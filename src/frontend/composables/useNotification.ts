import { ref } from 'vue'

// 单例实例
let notificationInstance: ReturnType<typeof createNotification> | null = null

// 检查浏览器是否支持 Notification API
function isNotificationSupported(): boolean {
  return typeof window !== 'undefined' && 'Notification' in window
}

function createNotification() {
  const notificationEnabled = ref(false)
  const permissionStatus = ref<NotificationPermission>('default')

  // 初始化通知权限状态
  function initPermission() {
    if (isNotificationSupported()) {
      permissionStatus.value = Notification.permission
    }
  }

  // 请求通知权限
  async function requestPermission(): Promise<boolean> {
    if (!isNotificationSupported()) {
      console.warn('浏览器不支持通知')
      return false
    }

    try {
      if (Notification.permission === 'granted') {
        permissionStatus.value = 'granted'
        return true
      }

      if (Notification.permission === 'denied') {
        permissionStatus.value = 'denied'
        return false
      }

      const permission = await Notification.requestPermission()
      permissionStatus.value = permission
      return permission === 'granted'
    }
    catch (e) {
      console.warn('请求通知权限失败:', e)
      return false
    }
  }

  // 切换通知开关
  async function toggleNotification(): Promise<boolean> {
    if (notificationEnabled.value) {
      // 关闭通知
      notificationEnabled.value = false
      saveToStorage()
      return true
    }

    // 开启通知 - 先请求权限
    const granted = await requestPermission()
    if (granted) {
      notificationEnabled.value = true
      saveToStorage()
      // 发送测试通知
      sendNotification('iterate 通知已开启', '当有新消息时会通知您')
    }
    return granted
  }

  // 发送通知
  function sendNotification(title: string, body?: string, options?: NotificationOptions) {
    if (!notificationEnabled.value) {
      return
    }
    if (!isNotificationSupported()) {
      return
    }
    if (Notification.permission !== 'granted') {
      return
    }

    try {
      const notification = new Notification(title, {
        body,
        icon: '/icons/icon-128.png',
        badge: '/icons/icon-128.png',
        tag: 'iterate-notification',
        ...options,
      })

      // 点击通知时聚焦窗口
      notification.onclick = () => {
        window.focus()
        notification.close()
      }

      return notification
    }
    catch (e) {
      console.warn('发送通知失败:', e)
      return undefined
    }
  }

  // 保存状态到 localStorage
  function saveToStorage() {
    try {
      localStorage.setItem('iterate-notification-enabled', JSON.stringify(notificationEnabled.value))
    }
    catch (e) {
      console.warn('保存通知设置失败:', e)
    }
  }

  // 从 localStorage 加载状态
  function loadFromStorage() {
    try {
      const saved = localStorage.getItem('iterate-notification-enabled')
      if (saved !== null) {
        notificationEnabled.value = JSON.parse(saved)
      }
    }
    catch (e) {
      console.warn('加载通知设置失败:', e)
    }
  }

  // 初始化
  function init() {
    initPermission()
    loadFromStorage()

    // 如果之前开启了通知但权限被撤销，自动关闭
    if (notificationEnabled.value && isNotificationSupported() && Notification.permission !== 'granted') {
      notificationEnabled.value = false
      saveToStorage()
    }
  }

  return {
    notificationEnabled,
    permissionStatus,
    toggleNotification,
    sendNotification,
    requestPermission,
    init,
  }
}

export function useNotification() {
  if (!notificationInstance) {
    notificationInstance = createNotification()
  }
  return notificationInstance
}
