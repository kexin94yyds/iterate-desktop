/**
 * 过滤自动触发的模板文字（与 Rust strip_auto_prompt 保持一致）
 * 用于：点击时间线节点后填充输入框时，过滤掉上下文追加内容
 */
export function stripAutoPrompt(input: string): string {
  const EXACT_SENTINELS = ['<!-- CONTEXT_INJECTION_START -->', '<!-- AUTO_PROMPT_START -->']
  const LINE_MARKERS = [
    '✔️不明白的地方反问我',
    '✔️继续调用 zhi',
    '✔️请记住',
    '✔继续调用 zhi',
    '快捷触发词',
  ]

  // 找精确 sentinel 的最早位置
  let cut: number | null = null
  for (const sentinel of EXACT_SENTINELS) {
    const pos = input.indexOf(sentinel)
    if (pos !== -1 && (cut === null || pos < cut))
      cut = pos
  }

  // 找行首 marker 的最早位置
  const lines = input.split('\n')
  let offset = 0
  for (const line of lines) {
    const trimmed = line.trimStart()
    if (LINE_MARKERS.some(m => trimmed.startsWith(m))) {
      if (cut === null || offset < cut)
        cut = offset
      break
    }
    offset += line.length + 1 // +1 for \n
  }

  if (cut !== null)
    return input.slice(0, cut).trimEnd()
  return input
}
