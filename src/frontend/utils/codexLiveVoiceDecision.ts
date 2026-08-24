const EXECUTION_QUESTIONS = [
  '需求已经确认是否现在开始执行',
]

const AFFIRMATIVE_DECISIONS = [
  '可以',
  '可以的',
  '好',
  '好的',
  '行',
  '确认',
  '确认执行',
  '我确认执行',
  '开始',
  '开始吧',
  '开始执行',
  '开始执行吧',
  '你开始吧',
  '那就开始吧',
  '现在执行',
  '直接执行',
  '直接做吧',
  '你直接做吧',
  '执行吧',
  '动手吧',
  '就这么做',
  '就这么做吧',
  '就这样做',
  '就这样做吧',
  '那就这么做',
  '那就这样做',
  '去做吧',
]

const NEGATIVE_DECISIONS = [
  '不',
  '不要',
  '不用',
  '不可以',
  '取消',
  '等等',
  '等一下',
  '先等等',
  '不要执行',
  '先不要执行',
  '先不执行',
  '暂不执行',
  '再商量一下',
  '我再想想',
]

const DIRECT_EXECUTION_COMMANDS = [
  '确认执行',
  '我确认执行',
  '开始执行',
  '开始执行吧',
  '现在执行',
  '直接执行',
]

const EXPLICIT_EXECUTION_REQUEST_MARKERS = [
  '调研',
  '研究',
  '整理',
  '分析',
  '汇总',
  '帮我查',
  '帮我搜',
  '查一下',
  '搜一下',
  '搜索',
  '查找',
  '联网查',
  '帮我读取',
  '读取文件',
  '帮我写',
  '写入文件',
  '创建文件',
  '新建文件',
  '修改文件',
  '改一下',
  '帮我改',
  '帮我修',
  '修复一下',
  '帮我安装',
  '安装一下',
  '帮我运行',
  '运行一下',
  '帮我执行',
  '执行一下',
  '帮我验证',
  '验证一下',
  '帮我测试',
  '测试一下',
  '帮我打开',
  '帮我截图',
  '调用hui',
  '使用hui',
  '按照hui',
  '调用xi',
  '使用xi',
  '按照xi',
  'searchfor',
  'lookfor',
  'writefile',
  'modifyfile',
  'pleasefix',
  'fixthe',
  'pleaseinstall',
  'installthe',
  'pleaserun',
  'runthe',
  'pleasetest',
  'testthe',
  'pleaseverify',
  'verifythe',
  'research',
  'analyze',
  'organize',
  'summarize',
]

const DIRECT_ACTION_MARKERS = [
  '继续',
  '接着',
  '看看',
  '看一下',
  '检查',
  '处理',
  '解决',
  '修复',
  '修改',
  '改成',
  '改为',
  '删除',
  '移除',
  '新增',
  '加上',
  '创建',
  '生成',
  '导出',
  '安装',
  '重启',
  '构建',
  '提交',
  '推送',
  '运行',
  '执行',
  '测试',
  '验证',
  '打开',
  '关闭',
  '截图',
]

const DISCUSSION_QUESTION_MARKERS = [
  '为什么',
  '怎么回事',
  '什么问题',
  '是什么',
  '什么意思',
  'what',
  'why',
  'how',
]

const EXECUTION_DELEGATION_CLARIFICATION_MARKERS = [
  '子代理',
  'subagent',
  'worker',
]

const CAPABILITY_QUESTION_MARKERS = [
  '能不能',
  '可不可以',
  '可以吗',
  '能否',
  '是否可以',
  '会不会',
  '有没有权限',
  '有权限吗',
  'canyou',
  'areyouable',
  'doyouhavepermission',
]

const EXACT_KNOWLEDGE_RECALL_REQUESTS = [
  'hui',
  'hui1',
  'hui0',
  '回',
  'xi',
  '习',
]

const KNOWLEDGE_RECALL_REQUEST_MARKERS = [
  '回溯',
  '刚刚讲了什么',
  '刚才说了什么',
  '上次讨论了什么',
  '之前做了什么',
  '项目进度',
  '昨天进行到哪',
  '昨天做到哪',
  '进行到哪',
  '做到哪',
  '经验库',
  '全局知识库',
  '上下文记忆',
  '恢复上下文',
]

export type CodexLiveVoiceGateDecision = 'confirm' | 'decline' | 'none'

export function normalizeCodexLiveVoiceDecision(text: string): string {
  return text
    .toLocaleLowerCase()
    .replace(/[\s\p{P}\p{S}]+/gu, '')
}

export function isCodexLiveExecutionQuestion(text: string): boolean {
  const normalized = normalizeCodexLiveVoiceDecision(text)
  return EXECUTION_QUESTIONS.includes(normalized)
}

export function isCodexLiveAffirmativeDecision(text: string): boolean {
  return AFFIRMATIVE_DECISIONS.includes(normalizeCodexLiveVoiceDecision(text))
}

export function isCodexLiveNegativeDecision(text: string): boolean {
  return NEGATIVE_DECISIONS.includes(normalizeCodexLiveVoiceDecision(text))
}

export function isDirectCodexLiveExecutionCommand(text: string): boolean {
  return DIRECT_EXECUTION_COMMANDS.includes(normalizeCodexLiveVoiceDecision(text))
}

export function isExplicitCodexLiveExecutionRequest(text: string): boolean {
  const normalized = normalizeCodexLiveVoiceDecision(text)
  if (!normalized || isCodexLiveNegativeDecision(normalized) || isDirectCodexLiveExecutionCommand(normalized))
    return false
  if (normalized.startsWith('不要') || normalized.startsWith('不用') || normalized.startsWith('别'))
    return false
  if (CAPABILITY_QUESTION_MARKERS.some(marker => normalized.includes(marker)))
    return false
  if (EXACT_KNOWLEDGE_RECALL_REQUESTS.includes(normalized))
    return true
  if (KNOWLEDGE_RECALL_REQUEST_MARKERS.some(marker => normalized.includes(marker)))
    return true
  if (EXPLICIT_EXECUTION_REQUEST_MARKERS.some(marker => normalized.includes(marker)))
    return true
  if (DISCUSSION_QUESTION_MARKERS.some(marker => normalized.includes(marker)))
    return false
  return DIRECT_ACTION_MARKERS.some(marker => normalized.includes(marker))
}

export function isCodexLiveExecutionDelegationClarification(text: string): boolean {
  const normalized = normalizeCodexLiveVoiceDecision(text)
  if (!normalized || isCodexLiveNegativeDecision(normalized))
    return false
  if (CAPABILITY_QUESTION_MARKERS.some(marker => normalized.includes(marker)))
    return false
  return EXECUTION_DELEGATION_CLARIFICATION_MARKERS.some(marker => normalized.includes(marker))
}

export function advanceCodexLiveExplicitExecutionRequestPending(
  current: boolean,
  text: string,
): boolean {
  if (isCodexLiveNegativeDecision(text))
    return false
  if (isDirectCodexLiveExecutionCommand(text) || isCodexLiveAffirmativeDecision(text))
    return current
  if (isExplicitCodexLiveExecutionRequest(text))
    return true
  if (current && isCodexLiveExecutionDelegationClarification(text))
    return true
  return false
}

export function resolveCodexLiveVoiceGateDecision(input: {
  awaitingConfirmation: boolean
  explicitRequestPending?: boolean
  priorUserUtteranceCount: number
  text: string
}): CodexLiveVoiceGateDecision {
  if (input.awaitingConfirmation) {
    if (isCodexLiveNegativeDecision(input.text))
      return 'decline'
    if (input.priorUserUtteranceCount > 0 && isCodexLiveAffirmativeDecision(input.text))
      return 'confirm'
    return 'none'
  }
  if (input.explicitRequestPending
    && input.priorUserUtteranceCount > 0
    && isDirectCodexLiveExecutionCommand(input.text)) {
    return 'confirm'
  }
  return 'none'
}
