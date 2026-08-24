<script setup lang="ts">
import { useTheme } from '../../composables/useTheme'
import { semanticColors } from '../../theme/colors'

const { currentTheme, setTheme } = useTheme()

const themeOptions = [
  { value: 'light', label: '浅色主题', icon: '☀️' },
  { value: 'dark', label: '深色主题', icon: '🌙' },
]

const colorPalettes = [
  { name: 'Primary', key: 'primary', colors: semanticColors.primary },
  { name: 'Gray', key: 'gray', colors: semanticColors.gray },
]

const themeVariables = [
  { name: '--primary-color', description: '主色调' },
  { name: '--body-color', description: '背景色' },
  { name: '--card-color', description: '卡片背景色' },
  { name: '--text-color', description: '文字颜色' },
  { name: '--border-color', description: '边框颜色' },
]

function handleThemeChange(theme: string) {
  setTheme(theme)
}

function getVariableValue(variable: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(variable) || 'undefined'
}

function copyColorValue(color: string) {
  navigator.clipboard.writeText(color)
  // 使用 console.log 替代 alert，避免阻塞用户界面
  console.log(`已复制颜色值: ${color}`)
}

// 移除系统偏好相关代码
</script>

<template>
  <div class="themes-test">
    <n-card title="主题测试 - 真实主题系统">
      <template #header-extra>
        <n-space>
          <n-tag size="small" type="info">
            引用: useTheme hook
          </n-tag>
          <n-tag size="small" :type="currentTheme === 'dark' ? 'warning' : 'info'">
            当前: {{ currentTheme }}
          </n-tag>
        </n-space>
      </template>

      <n-space vertical size="large">
        <!-- 主题切换 -->
        <div class="theme-section">
          <h3 class="section-title">
            主题切换
          </h3>

          <n-card size="small">
            <n-space>
              <n-button
                v-for="theme in themeOptions" :key="theme.value"
                :type="currentTheme === theme.value ? 'primary' : 'default'" size="small"
                @click="handleThemeChange(theme.value)"
              >
                <template #icon>
                  <span>{{ theme.icon }}</span>
                </template>
                {{ theme.label }}
              </n-button>
            </n-space>

            <div class="theme-info">
              <n-space vertical size="small">
                <div><strong>当前主题:</strong> {{ currentTheme }}</div>
                <div><strong>实际应用:</strong> {{ currentTheme }}</div>
              </n-space>
            </div>
          </n-card>
        </div>

        <!-- 颜色调色板 -->
        <div class="theme-section">
          <h3 class="section-title">
            颜色调色板 (真实配置)
          </h3>

          <n-space vertical>
            <div v-for="palette in colorPalettes" :key="palette.key" class="color-palette">
              <h4 class="palette-title">
                {{ palette.name }}
              </h4>
              <div class="color-grid">
                <div
                  v-for="(color, shade) in palette.colors" :key="shade" class="color-item"
                  :style="{ backgroundColor: color }" :title="`${palette.name}-${shade}: ${color}`"
                  @click="copyColorValue(color)"
                >
                  <span class="color-label">{{ shade }}</span>
                  <span class="color-value">{{ color }}</span>
                </div>
              </div>
            </div>
          </n-space>
        </div>

        <!-- CSS 变量 -->
        <div class="theme-section">
          <h3 class="section-title">
            CSS 变量 (实时值)
          </h3>

          <n-card size="small">
            <div class="variables-grid">
              <div v-for="variable in themeVariables" :key="variable.name" class="variable-item">
                <div class="variable-name">
                  {{ variable.name }}
                </div>
                <div class="variable-description">
                  {{ variable.description }}
                </div>
                <div class="variable-value">
                  {{ getVariableValue(variable.name) }}
                </div>
                <div
                  class="variable-preview" :style="{ backgroundColor: getVariableValue(variable.name) }"
                  @click="copyColorValue(getVariableValue(variable.name))"
                />
              </div>
            </div>
          </n-card>
        </div>

        <!-- 组件主题预览 -->
        <div class="theme-section">
          <h3 class="section-title">
            组件主题预览
          </h3>

          <n-card size="small">
            <n-space vertical>
              <!-- 按钮组 -->
              <div class="component-preview">
                <h4>按钮组件</h4>
                <n-space>
                  <n-button size="small">
                    默认
                  </n-button>
                  <n-button type="primary" size="small">
                    主要
                  </n-button>
                  <n-button type="success" size="small">
                    成功
                  </n-button>
                  <n-button type="warning" size="small">
                    警告
                  </n-button>
                  <n-button type="error" size="small">
                    错误
                  </n-button>
                </n-space>
              </div>

              <!-- 输入组件 -->
              <div class="component-preview">
                <h4>输入组件</h4>
                <n-space>
                  <n-input size="small" placeholder="普通输入框" style="width: 150px;" />
                  <n-switch size="small" />
                  <n-checkbox size="small">
                    复选框
                  </n-checkbox>
                </n-space>
              </div>

              <!-- 标签组件 -->
              <div class="component-preview">
                <h4>标签组件</h4>
                <n-space>
                  <n-tag size="small">
                    默认
                  </n-tag>
                  <n-tag type="primary" size="small">
                    主要
                  </n-tag>
                  <n-tag type="success" size="small">
                    成功
                  </n-tag>
                  <n-tag type="warning" size="small">
                    警告
                  </n-tag>
                  <n-tag type="error" size="small">
                    错误
                  </n-tag>
                </n-space>
              </div>

              <!-- 卡片组件 -->
              <div class="component-preview">
                <h4>卡片组件</h4>
                <n-card title="示例卡片" size="small" style="max-width: 300px;">
                  <p>这是一个示例卡片，用于展示当前主题下的卡片样式。</p>
                  <template #footer>
                    <n-button size="small">
                      操作
                    </n-button>
                  </template>
                </n-card>
              </div>
            </n-space>
          </n-card>
        </div>

        <!-- 主题文件信息 -->
        <div class="theme-section">
          <h3 class="section-title">
            主题文件信息
          </h3>

          <n-card size="small">
            <n-space vertical>
              <div><strong>主题 Hook:</strong> src/frontend/hooks/useTheme.ts</div>
              <div><strong>主题配置:</strong> src/frontend/theme/index.ts</div>
              <div><strong>颜色配置:</strong> src/frontend/theme/colors.ts</div>
              <div><strong>UnoCSS 配置:</strong> uno.config.ts</div>
              <div>
                <strong>当前使用:</strong> {{ currentTheme }}
              </div>
            </n-space>
          </n-card>
        </div>
      </n-space>
    </n-card>
  </div>
</template>

<style scoped>
.themes-test {
  max-width: 1000px;
  margin: 0 auto;
}

.theme-section {
  margin-bottom: 30px;
}

.section-title {
  margin: 0 0 15px 0;
  color: var(--text-color);
  font-size: 1.1rem;
  font-weight: 600;
  border-bottom: 2px solid var(--primary-color);
  padding-bottom: 5px;
  display: inline-block;
}

.theme-info {
  margin-top: 15px;
  padding-top: 15px;
  border-top: 1px solid var(--border-color);
}

.color-palette {
  margin-bottom: 20px;
}

.palette-title {
  margin: 0 0 10px 0;
  color: var(--text-color);
  font-size: 1rem;
  font-weight: 500;
}

.color-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: 10px;
}

.color-item {
  height: 80px;
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: space-between;
  padding: 8px;
  border: 1px solid var(--border-color);
  cursor: pointer;
  transition: transform 0.2s;
}

.color-item:hover {
  transform: scale(1.05);
}

.color-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: white;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.7);
  background: rgba(0, 0, 0, 0.4);
  padding: 2px 6px;
  border-radius: 3px;
}

.color-value {
  font-size: 0.7rem;
  font-family: monospace;
  color: white;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.7);
  background: rgba(0, 0, 0, 0.4);
  padding: 2px 4px;
  border-radius: 3px;
}

.variables-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 15px;
}

.variable-item {
  padding: 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--card-color);
  cursor: pointer;
  transition: transform 0.2s;
}

.variable-item:hover {
  transform: scale(1.02);
}

.variable-name {
  font-family: monospace;
  font-weight: 600;
  color: var(--primary-color);
  margin-bottom: 4px;
}

.variable-description {
  font-size: 0.85rem;
  color: var(--text-color);
  opacity: 0.8;
  margin-bottom: 8px;
}

.variable-value {
  font-family: monospace;
  font-size: 0.8rem;
  color: var(--text-color);
  opacity: 0.7;
  margin-bottom: 8px;
}

.variable-preview {
  height: 20px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
}

.component-preview {
  padding: 15px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--card-color);
}

.component-preview h4 {
  margin: 0 0 10px 0;
  color: var(--text-color);
  font-size: 0.9rem;
  font-weight: 500;
}
</style>
