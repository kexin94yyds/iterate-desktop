<script setup lang="ts">
import FeatureCard from '../common/FeatureCard.vue'
import ProjectInfoCard from '../common/ProjectInfoCard.vue'

const features = [
  {
    icon: 'i-carbon-chat text-lg text-blue-600 dark:text-blue-400',
    title: '智 · Zhi',
    subtitle: 'Smart Interception',
    iconBg: 'bg-blue-100',
    features: [
      '核心拦截交互，保护对话连贯性',
      '支持文本和图片多模态输入',
      '预定义选项快速响应',
      'Markdown 渲染',
      '跨平台快捷键支持',
    ],
  },
  {
    icon: 'i-carbon-data-base text-lg text-purple-600 dark:text-purple-400',
    title: '记 · Ji',
    subtitle: 'Memory Engine',
    iconBg: 'bg-purple-100',
    features: [
      '按项目独立存储，持久记忆',
      '规则、偏好、笔记、上下文四类',
      '跨会话不失忆',
      '知识库同步到 GitHub',
      '沉淀问题与最佳实践',
    ],
  },
  {
    icon: 'i-carbon-search text-lg text-green-600 dark:text-green-400',
    title: '搜 · Sou',
    subtitle: 'Semantic Code Search',
    iconBg: 'bg-green-100',
    features: [
      '基于 acemcp 的向量搜索',
      '增量索引支持',
      '多文件类型支持',
      '自定义过滤规则',
      '快速代码定位',
    ],
  },
  {
    icon: 'i-carbon-bot text-lg text-orange-600 dark:text-orange-400',
    title: '派 · Pai',
    subtitle: 'Agent Dispatcher',
    iconBg: 'bg-orange-100',
    features: [
      '将复杂任务拆分为 room worker',
      '使用 codex-room 调度当前项目等待态 AI',
      '并发任务编排',
      '结构化回包和汇总',
      '主会话统一 zhi 收口',
    ],
  },
  {
    icon: 'i-carbon-book text-lg text-cyan-600 dark:text-cyan-400',
    title: '习 · Xi',
    subtitle: 'Best Practices',
    iconBg: 'bg-cyan-100',
    features: [
      '检索历史最佳实践',
      '让 AI 学习您的工作模式',
      '搜索 problems / patterns',
      '跨项目经验复用',
      '本地知识库驱动',
    ],
  },
  {
    icon: 'i-carbon-catalog text-lg text-indigo-600 dark:text-indigo-400',
    title: '词 · Ci',
    subtitle: 'Prompt Library',
    iconBg: 'bg-indigo-100',
    features: [
      '快速检索提示词模板',
      '分目录管理，随取随用',
      '支持 skills / workflows',
      '触发词自动匹配',
      '本地 .cunzhi-knowledge 驱动',
    ],
  },
]
</script>

<template>
  <div class="tab-content">
    <!-- 项目信息卡片 -->
    <div class="mb-5">
      <ProjectInfoCard />
    </div>

    <!-- 功能卡片 -->
    <n-grid :cols="2" :x-gap="20" :y-gap="20" responsive="screen">
      <n-grid-item
        v-for="(feature, index) in features"
        :key="index"
        :span="1"
      >
        <FeatureCard :feature="feature" />
      </n-grid-item>
    </n-grid>
  </div>
</template>
