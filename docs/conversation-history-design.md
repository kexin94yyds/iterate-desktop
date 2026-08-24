# 对话历史与节点撤回功能设计

## 一、数据结构设计

### 1. 对话节点（ConversationNode）

```typescript
interface ConversationNode {
  id: string                    // 节点唯一ID（UUID）
  parent_id: string | null      // 父节点ID（null 表示根节点）
  timestamp: string             // ISO 8601 时间戳
  type: 'user' | 'assistant'    // 节点类型
  content: string               // 消息内容
  is_markdown: boolean          // 是否为 Markdown 格式
  metadata: NodeMetadata        // 元数据
}

interface NodeMetadata {
  project_path?: string
  predefined_options?: string[]
  selected_option?: string
  images?: ImageAttachment[]
  link_url?: string
  link_title?: string
}
```

### 2. 对话树（ConversationTree）

```typescript
interface ConversationTree {
  id: string                    // 对话树ID
  created_at: string
  updated_at: string
  current_node_id: string       // 当前激活的节点ID
  nodes: Map<string, ConversationNode>
  branches: Map<string, string[]> // parent_id -> [child_ids]
}
```

### 3. 存储结构（Rust）

```rust
// src/rust/conversation/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub node_type: NodeType,
    pub content: String,
    pub is_markdown: bool,
    pub metadata: NodeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub project_path: Option<String>,
    pub predefined_options: Option<Vec<String>>,
    pub selected_option: Option<String>,
    pub images: Option<Vec<ImageAttachment>>,
    pub link_url: Option<String>,
    pub link_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTree {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub current_node_id: String,
    pub nodes: HashMap<String, ConversationNode>,
    pub branches: HashMap<String, Vec<String>>, // parent_id -> child_ids
}
```

---

## 二、核心功能实现

### 1. 对话历史记录

#### Rust 端（src/rust/conversation/manager.rs）

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct ConversationManager {
    trees: Arc<RwLock<HashMap<String, ConversationTree>>>,
    current_tree_id: Arc<RwLock<Option<String>>>,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self {
            trees: Arc::new(RwLock::new(HashMap::new())),
            current_tree_id: Arc::new(RwLock::new(None)),
        }
    }

    /// 创建新对话树
    pub async fn create_tree(&self) -> String {
        let tree_id = Uuid::new_v4().to_string();
        let tree = ConversationTree {
            id: tree_id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            current_node_id: String::new(),
            nodes: HashMap::new(),
            branches: HashMap::new(),
        };
        
        let mut trees = self.trees.write().await;
        trees.insert(tree_id.clone(), tree);
        
        let mut current = self.current_tree_id.write().await;
        *current = Some(tree_id.clone());
        
        tree_id
    }

    /// 添加节点
    pub async fn add_node(
        &self,
        tree_id: &str,
        parent_id: Option<String>,
        node_type: NodeType,
        content: String,
        is_markdown: bool,
        metadata: NodeMetadata,
    ) -> Result<String, String> {
        let node_id = Uuid::new_v4().to_string();
        let node = ConversationNode {
            id: node_id.clone(),
            parent_id: parent_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            node_type,
            content,
            is_markdown,
            metadata,
        };

        let mut trees = self.trees.write().await;
        let tree = trees.get_mut(tree_id)
            .ok_or_else(|| "Tree not found".to_string())?;

        // 添加节点
        tree.nodes.insert(node_id.clone(), node);

        // 更新分支
        if let Some(pid) = parent_id {
            tree.branches.entry(pid)
                .or_insert_with(Vec::new)
                .push(node_id.clone());
        }

        // 更新当前节点
        tree.current_node_id = node_id.clone();
        tree.updated_at = chrono::Utc::now().to_rfc3339();

        Ok(node_id)
    }

    /// 切换到指定节点（撤回功能）
    pub async fn switch_to_node(
        &self,
        tree_id: &str,
        node_id: &str,
    ) -> Result<ConversationNode, String> {
        let mut trees = self.trees.write().await;
        let tree = trees.get_mut(tree_id)
            .ok_or_else(|| "Tree not found".to_string())?;

        let node = tree.nodes.get(node_id)
            .ok_or_else(|| "Node not found".to_string())?
            .clone();

        tree.current_node_id = node_id.to_string();
        tree.updated_at = chrono::Utc::now().to_rfc3339();

        Ok(node)
    }

    /// 获取节点路径（从根到当前节点）
    pub async fn get_node_path(
        &self,
        tree_id: &str,
        node_id: &str,
    ) -> Result<Vec<ConversationNode>, String> {
        let trees = self.trees.read().await;
        let tree = trees.get(tree_id)
            .ok_or_else(|| "Tree not found".to_string())?;

        let mut path = Vec::new();
        let mut current_id = Some(node_id.to_string());

        while let Some(id) = current_id {
            let node = tree.nodes.get(&id)
                .ok_or_else(|| format!("Node {} not found", id))?;
            path.push(node.clone());
            current_id = node.parent_id.clone();
        }

        path.reverse();
        Ok(path)
    }

    /// 导出对话树（用于文件导出）
    pub async fn export_tree(&self, tree_id: &str) -> Result<ConversationTree, String> {
        let trees = self.trees.read().await;
        trees.get(tree_id)
            .cloned()
            .ok_or_else(|| "Tree not found".to_string())
    }

    /// 导入对话树（用于文件导入）
    pub async fn import_tree(&self, tree: ConversationTree) -> Result<String, String> {
        let tree_id = tree.id.clone();
        let mut trees = self.trees.write().await;
        trees.insert(tree_id.clone(), tree);
        Ok(tree_id)
    }
}
```

#### Tauri Commands（src/rust/conversation/commands.rs）

```rust
use tauri::State;

#[tauri::command]
pub async fn create_conversation_tree(
    manager: State<'_, Arc<ConversationManager>>,
) -> Result<String, String> {
    manager.create_tree().await
}

#[tauri::command]
pub async fn add_conversation_node(
    manager: State<'_, Arc<ConversationManager>>,
    tree_id: String,
    parent_id: Option<String>,
    node_type: String,
    content: String,
    is_markdown: bool,
    metadata: NodeMetadata,
) -> Result<String, String> {
    let node_type = match node_type.as_str() {
        "user" => NodeType::User,
        "assistant" => NodeType::Assistant,
        _ => return Err("Invalid node type".to_string()),
    };

    manager.add_node(&tree_id, parent_id, node_type, content, is_markdown, metadata).await
}

#[tauri::command]
pub async fn switch_conversation_node(
    manager: State<'_, Arc<ConversationManager>>,
    tree_id: String,
    node_id: String,
) -> Result<ConversationNode, String> {
    manager.switch_to_node(&tree_id, &node_id).await
}

#[tauri::command]
pub async fn get_conversation_path(
    manager: State<'_, Arc<ConversationManager>>,
    tree_id: String,
    node_id: String,
) -> Result<Vec<ConversationNode>, String> {
    manager.get_node_path(&tree_id, &node_id).await
}

#[tauri::command]
pub async fn export_conversation(
    manager: State<'_, Arc<ConversationManager>>,
    tree_id: String,
) -> Result<ConversationTree, String> {
    manager.export_tree(&tree_id).await
}

#[tauri::command]
pub async fn import_conversation(
    manager: State<'_, Arc<ConversationManager>>,
    tree: ConversationTree,
) -> Result<String, String> {
    manager.import_tree(tree).await
}
```

---

### 2. 前端时间线 UI

#### Vue 组件（src/frontend/components/conversation/TimelineView.vue）

```vue
<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'

interface ConversationNode {
  id: string
  parent_id: string | null
  timestamp: string
  node_type: 'user' | 'assistant'
  content: string
  is_markdown: boolean
}

const props = defineProps<{
  treeId: string
  currentNodeId: string
}>()

const emit = defineEmits<{
  nodeClick: [nodeId: string]
}>()

const nodes = ref<ConversationNode[]>([])
const loading = ref(false)

// 加载对话路径
async function loadPath() {
  loading.value = true
  try {
    const path = await invoke<ConversationNode[]>('get_conversation_path', {
      treeId: props.treeId,
      nodeId: props.currentNodeId,
    })
    nodes.value = path
  }
  catch (error) {
    console.error('加载对话路径失败:', error)
  }
  finally {
    loading.value = false
  }
}

// 节点点击处理
function handleNodeClick(nodeId: string) {
  emit('nodeClick', nodeId)
}

// 格式化时间
function formatTime(timestamp: string) {
  const date = new Date(timestamp)
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
}

onMounted(() => {
  loadPath()
})
</script>

<template>
  <div class="timeline-container">
    <div class="timeline-header">
      <div class="text-sm font-medium">
        对话历史
      </div>
      <div class="text-xs opacity-60">
        共 {{ nodes.length }} 个节点
      </div>
    </div>

    <div v-if="loading" class="timeline-loading">
      加载中...
    </div>

    <div v-else class="timeline-nodes">
      <div
        v-for="(node, index) in nodes"
        :key="node.id"
        class="timeline-node"
        :class="{
          'node-user': node.node_type === 'user',
          'node-assistant': node.node_type === 'assistant',
          'node-current': node.id === currentNodeId,
        }"
        @click="handleNodeClick(node.id)"
      >
        <!-- 连接线 -->
        <div v-if="index > 0" class="node-connector" />

        <!-- 节点图标 -->
        <div class="node-icon">
          <div
            :class="node.node_type === 'user' ? 'i-carbon-user' : 'i-carbon-chat-bot'"
            class="w-4 h-4"
          />
        </div>

        <!-- 节点内容 -->
        <div class="node-content">
          <div class="node-time">
            {{ formatTime(node.timestamp) }}
          </div>
          <div class="node-text">
            {{ node.content.slice(0, 50) }}{{ node.content.length > 50 ? '...' : '' }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.timeline-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-color);
}

.timeline-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
}

.timeline-nodes {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.timeline-node {
  position: relative;
  display: flex;
  gap: 12px;
  padding: 8px;
  margin-bottom: 8px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.timeline-node:hover {
  background: var(--hover-bg);
}

.timeline-node.node-current {
  background: var(--primary-bg);
  border: 2px solid var(--primary-color);
}

.node-connector {
  position: absolute;
  left: 19px;
  top: -8px;
  width: 2px;
  height: 8px;
  background: var(--border-color);
}

.node-icon {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.node-user .node-icon {
  background: #3b82f6;
  color: white;
}

.node-assistant .node-icon {
  background: #10b981;
  color: white;
}

.node-content {
  flex: 1;
  min-width: 0;
}

.node-time {
  font-size: 11px;
  opacity: 0.6;
  margin-bottom: 4px;
}

.node-text {
  font-size: 13px;
  line-height: 1.4;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
```

---

## 三、文件导入/导出功能

### 导出格式

```json
{
  "format": "iterate.conversation.v1",
  "version": "1.0.0",
  "exported_at": "2026-02-16T07:53:00Z",
  "tree": {
    "id": "uuid",
    "created_at": "2026-02-16T07:00:00Z",
    "updated_at": "2026-02-16T07:53:00Z",
    "current_node_id": "node-uuid",
    "nodes": {
      "node-uuid-1": { ... },
      "node-uuid-2": { ... }
    },
    "branches": {
      "parent-uuid": ["child-uuid-1", "child-uuid-2"]
    }
  }
}
```

### Rust 导出实现

```rust
use std::fs::File;
use std::io::Write;
use serde_json;

#[tauri::command]
pub async fn export_conversation_to_file(
    manager: State<'_, Arc<ConversationManager>>,
    tree_id: String,
    file_path: String,
) -> Result<(), String> {
    let tree = manager.export_tree(&tree_id).await?;
    
    let export_data = serde_json::json!({
        "format": "iterate.conversation.v1",
        "version": "1.0.0",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "tree": tree,
    });

    let json = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("JSON serialization failed: {}", e))?;

    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn import_conversation_from_file(
    manager: State<'_, Arc<ConversationManager>>,
    file_path: String,
) -> Result<String, String> {
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // 验证格式
    let format = data.get("format")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing format field".to_string())?;

    if format != "iterate.conversation.v1" {
        return Err(format!("Unsupported format: {}", format));
    }

    // 提取对话树
    let tree: ConversationTree = serde_json::from_value(
        data.get("tree")
            .ok_or_else(|| "Missing tree field".to_string())?
            .clone()
    ).map_err(|e| format!("Invalid tree data: {}", e))?;

    manager.import_tree(tree).await
}
```

---

## 四、集成到现有系统

### 1. 修改 MCP 交互流程

在 `src/rust/bridge/ws.rs` 中集成对话历史记录：

```rust
// 接收用户输入时记录节点
async fn handle_user_input(
    conversation_manager: &ConversationManager,
    tree_id: &str,
    parent_id: Option<String>,
    user_input: String,
    metadata: NodeMetadata,
) -> Result<String, String> {
    conversation_manager.add_node(
        tree_id,
        parent_id,
        NodeType::User,
        user_input,
        false,
        metadata,
    ).await
}

// AI 响应时记录节点
async fn handle_assistant_response(
    conversation_manager: &ConversationManager,
    tree_id: &str,
    parent_id: String,
    response: String,
    is_markdown: bool,
) -> Result<String, String> {
    conversation_manager.add_node(
        tree_id,
        Some(parent_id),
        NodeType::Assistant,
        response,
        is_markdown,
        NodeMetadata::default(),
    ).await
}
```

### 2. 前端集成

在 `PopupInput.vue` 中添加时间线按钮：

```vue
<template>
  <div class="popup-container">
    <!-- 时间线按钮 -->
    <n-button
      circle
      quaternary
      size="small"
      title="查看对话历史"
      @click="showTimeline = !showTimeline"
    >
      <template #icon>
        <div class="i-carbon-tree-view w-4 h-4" />
      </template>
    </n-button>

    <!-- 时间线侧边栏 -->
    <n-drawer
      v-model:show="showTimeline"
      :width="300"
      placement="left"
    >
      <TimelineView
        :tree-id="currentTreeId"
        :current-node-id="currentNodeId"
        @node-click="handleNodeSwitch"
      />
    </n-drawer>
  </div>
</template>
```

---

## 五、实施步骤

### 阶段 1：数据层（P0）
1. 创建 Rust 数据结构（`src/rust/conversation/types.rs`）
2. 实现 `ConversationManager`（`src/rust/conversation/manager.rs`）
3. 添加 Tauri commands（`src/rust/conversation/commands.rs`）
4. 集成到 `main.rs`

### 阶段 2：文件导入/导出（P1）
1. 实现导出功能（JSON 格式）
2. 实现导入功能（格式校验 + 数据迁移）
3. 前端添加导入/导出按钮

### 阶段 3：时间线 UI（P2）
1. 创建 `TimelineView.vue` 组件
2. 集成到 `PopupInput.vue`
3. 实现节点切换交互

### 阶段 4：集成测试（P3）
1. 测试对话记录功能
2. 测试节点切换功能
3. 测试导入/导出功能
4. 性能测试（大量节点场景）

---

## 六、注意事项

### 性能优化
- 对话树超过 1000 个节点时考虑分页加载
- 使用虚拟滚动优化时间线渲染
- 定期清理旧对话树（可选）

### 数据安全
- 导入前验证 JSON 格式
- 防止恶意数据注入
- 敏感信息脱敏（如果需要）

### 用户体验
- 节点切换时平滑过渡动画
- 支持键盘快捷键（Ctrl+Z 撤回）
- 提供搜索功能（搜索历史消息）
