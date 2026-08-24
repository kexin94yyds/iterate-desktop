use anyhow::Result;
use chromiumoxide::{Browser, Page};
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};

use super::config::{match_ai_site, BrowserMonitorConfig};
use super::detector::{AiCompletionDetector, AiCompletionEvent, AiStatus, PageState};

/// 浏览器监控器
pub struct BrowserMonitor {
    config: BrowserMonitorConfig,
    browser: Option<Arc<Browser>>,
    monitored_pages: Arc<RwLock<Vec<MonitoredPage>>>,
    event_tx: broadcast::Sender<AiCompletionEvent>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

struct MonitoredPage {
    page: Page,
    state: PageState,
    detector: AiCompletionDetector,
    was_generating: bool,
}

impl BrowserMonitor {
    pub fn new(config: BrowserMonitorConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            config,
            browser: None,
            monitored_pages: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            shutdown_tx: None,
        }
    }

    /// 获取事件接收器
    pub fn subscribe(&self) -> broadcast::Receiver<AiCompletionEvent> {
        self.event_tx.subscribe()
    }

    /// 连接到 Chrome 浏览器
    pub async fn connect(&mut self) -> Result<()> {
        let debug_url = format!("http://127.0.0.1:{}", self.config.chrome_debug_port);

        log::info!("尝试连接到 Chrome 调试端口: {}", debug_url);

        let (browser, mut handler) = Browser::connect(&debug_url).await
            .map_err(|e| anyhow::anyhow!(
                "无法连接到 Chrome。请确保 Chrome 以调试模式启动：\n\
                macOS: /Applications/Google\\ Chrome.app/Contents/MacOS/Google\\ Chrome --remote-debugging-port={}\n\
                错误: {}", self.config.chrome_debug_port, e
            ))?;

        // 启动事件处理
        tokio::spawn(async move {
            while let Some(_event) = handler.next().await {
                // 处理浏览器事件
            }
        });

        self.browser = Some(Arc::new(browser));
        log::info!("成功连接到 Chrome 浏览器");

        Ok(())
    }

    /// 开始监控
    pub async fn start_monitoring(&mut self) -> Result<()> {
        let browser = self
            .browser
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未连接到浏览器"))?
            .clone();

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let monitored_pages = self.monitored_pages.clone();
        let event_tx = self.event_tx.clone();
        let poll_interval = Duration::from_millis(self.config.poll_interval_ms);

        // 启动监控任务
        tokio::spawn(async move {
            log::info!("浏览器监控已启动");

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        log::info!("浏览器监控已停止");
                        break;
                    }
                    _ = tokio::time::sleep(poll_interval) => {
                        // 刷新页面列表
                        if let Err(e) = Self::refresh_pages(&browser, &monitored_pages).await {
                            log::warn!("刷新页面列表失败: {}", e);
                        }

                        // 检查各页面状态
                        if let Err(e) = Self::check_pages(&monitored_pages, &event_tx).await {
                            log::warn!("检查页面状态失败: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// 停止监控
    pub async fn stop_monitoring(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    /// 刷新监控的页面列表
    async fn refresh_pages(
        browser: &Browser,
        monitored_pages: &Arc<RwLock<Vec<MonitoredPage>>>,
    ) -> Result<()> {
        let pages = browser.pages().await?;
        let mut monitored = monitored_pages.write().await;

        // 获取当前监控的 URL 列表
        let current_urls: Vec<String> = monitored.iter().map(|p| p.state.url.clone()).collect();

        for page in pages {
            if let Ok(Some(url)) = page.url().await {
                let url_str = url.as_str();

                // 检查是否是支持的 AI 网站
                if match_ai_site(url_str).is_some() {
                    // 检查是否已经在监控列表中
                    if !current_urls.contains(&url_str.to_string()) {
                        let title = page
                            .get_title()
                            .await
                            .unwrap_or_default()
                            .unwrap_or_default();
                        let state = PageState::new(url_str.to_string(), title);
                        let detector = AiCompletionDetector::new(url_str);

                        log::info!("发现新的 AI 页面: {} - {}", state.site_name, url_str);

                        monitored.push(MonitoredPage {
                            page,
                            state,
                            detector,
                            was_generating: false,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// 检查各页面的 AI 完成状态
    async fn check_pages(
        monitored_pages: &Arc<RwLock<Vec<MonitoredPage>>>,
        event_tx: &broadcast::Sender<AiCompletionEvent>,
    ) -> Result<()> {
        let mut pages = monitored_pages.write().await;

        for monitored_page in pages.iter_mut() {
            // 获取检测脚本
            let script = match monitored_page.detector.get_is_generating_script() {
                Ok(s) => s,
                Err(_) => continue,
            };

            // 执行检测脚本
            let is_generating = match monitored_page.page.evaluate(script).await {
                Ok(result) => result.into_value::<bool>().unwrap_or(false),
                Err(e) => {
                    log::debug!("执行检测脚本失败: {}", e);
                    false
                }
            };

            // 检测状态变化：从生成中变为完成
            if monitored_page.was_generating && !is_generating {
                log::info!("AI 完成生成: {}", monitored_page.state.url);

                // 获取最后一条消息预览
                let message_preview =
                    if let Ok(msg_script) = monitored_page.detector.get_last_message_script() {
                        monitored_page
                            .page
                            .evaluate(msg_script)
                            .await
                            .ok()
                            .and_then(|r| r.into_value::<String>().ok())
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };

                // 更新页面标题
                if let Ok(Some(title)) = monitored_page.page.get_title().await {
                    monitored_page.state.title = title;
                }

                // 发送完成事件
                let event = AiCompletionEvent::new(&monitored_page.state, message_preview);
                let _ = event_tx.send(event);

                monitored_page.state.status = AiStatus::Completed;
            } else if is_generating {
                monitored_page.state.status = AiStatus::Generating;
            } else {
                monitored_page.state.status = AiStatus::Idle;
            }

            monitored_page.was_generating = is_generating;
            monitored_page.state.last_check = chrono::Utc::now();
        }

        Ok(())
    }

    /// 获取所有监控页面的状态
    pub async fn get_page_states(&self) -> Vec<PageState> {
        let pages = self.monitored_pages.read().await;
        pages.iter().map(|p| p.state.clone()).collect()
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.browser.is_some()
    }
}

/// 全局浏览器监控器实例
static BROWSER_MONITOR: once_cell::sync::Lazy<Arc<RwLock<Option<BrowserMonitor>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

/// 获取全局浏览器监控器
pub async fn get_browser_monitor() -> Arc<RwLock<Option<BrowserMonitor>>> {
    BROWSER_MONITOR.clone()
}

/// 初始化并启动浏览器监控
pub async fn start_browser_monitor(
    config: BrowserMonitorConfig,
) -> Result<broadcast::Receiver<AiCompletionEvent>> {
    let mut monitor = BrowserMonitor::new(config);
    monitor.connect().await?;
    let receiver = monitor.subscribe();
    monitor.start_monitoring().await?;

    let mut global = BROWSER_MONITOR.write().await;
    *global = Some(monitor);

    Ok(receiver)
}

/// 停止浏览器监控
pub async fn stop_browser_monitor() -> Result<()> {
    let mut global = BROWSER_MONITOR.write().await;
    if let Some(mut monitor) = global.take() {
        monitor.stop_monitoring().await;
    }
    Ok(())
}
