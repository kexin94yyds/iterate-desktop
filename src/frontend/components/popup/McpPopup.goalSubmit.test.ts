/* eslint-disable test/no-import-node-test */
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { describe, it } from 'node:test'

const source = await readFile(new URL('./McpPopup.vue', import.meta.url), 'utf8')
const replySettingsSource = await readFile(new URL('../settings/ReplySettings.vue', import.meta.url), 'utf8')
const settingsTabSource = await readFile(new URL('../tabs/SettingsTab.vue', import.meta.url), 'utf8')
const mainLayoutSource = await readFile(new URL('../layout/MainLayout.vue', import.meta.url), 'utf8')

describe('McpPopup goal submit attachments', () => {
  it('allows a predefined-option-only reply without passing null into submit-source parsing', () => {
    assert.match(source, /return selectedOptions\.value\.length > 0/)
    assert.match(source, /resolveSubmitSource\(finalUserInput \?\? '', selectedOptions\.value\)/)
  })

  it('combines selected files into the GoalRun target without leaking image internals', () => {
    assert.match(source, /function buildGoalTargetText\(/)
    assert.match(source, /function buildSelectedOptionsContext\(/)
    assert.match(source, /function buildGoalSubmitPrompt\(/)
    assert.match(source, /先把这句话整理成可执行目标/)
    assert.match(source, /执行任何实现动作前/)
    assert.match(source, /get_goal/)
    assert.match(source, /create_goal/)
    assert.match(source, /update_goal 为 complete/)
    assert.match(source, /Codex 正式 Goal 是唯一状态源/)
    assert.match(source, /绝不能伪造完成或在未同步状态下继续/)
    assert.match(source, /goal_prompt_template/)
    assert.match(source, /## XI 启动检查（正式 Goal 同步后执行）/)
    assert.match(source, /任何实现动作前必须执行 xi/)
    assert.match(source, /禁止重复实现或伪造完成/)
    assert.match(source, /目标已登记到 Live Goal，正在提交给 Codex/)
    assert.doesNotMatch(source, /目标已同步到 Mac 菜单栏/)
    assert.match(source, /只有目标依赖历史/)
    assert.match(source, /timeline\/thread\/run/)
    assert.doesNotMatch(source, /先建立 Goal Spec/)
    assert.doesNotMatch(source, /success_criteria/)
    assert.doesNotMatch(source, /stop_conditions/)
    assert.match(source, /`选中的选项：\\n\$\{missingOptions\.map\(option => `- \$\{option\}`\)\.join\('\\n'\)\}`/)
    assert.match(source, /`相关文件：\\n\$\{fileRefs\}`/)
    assert.doesNotMatch(source, /附件地址：/)
    assert.doesNotMatch(source, /见 images 附件/)
    assert.doesNotMatch(source, /images\[\$\{index\}\]|images\[\$\{imageCount\}\]/)
    assert.match(source, /const goalText = buildGoalTargetText\(/)
  })

  it('keeps goal image attachments instead of sending an empty image list', () => {
    assert.match(source, /function buildImageAttachments\(/)
    assert.match(source, /images: buildImageAttachments\(draggedImages\.value\)/)
    assert.doesNotMatch(source, /source: 'popup_goal_submit'[\s\S]{0,240}images: \[\]/)
  })

  it('moves Goal configuration into settings and removes the standalone Goal tab', () => {
    assert.match(replySettingsSource, /Goal 模板/)
    assert.match(replySettingsSource, /goal_prompt_template/)
    assert.match(replySettingsSource, /目标内容与 xi 去重检查由系统自动加入/)
    assert.doesNotMatch(replySettingsSource, /<!-- 继续提示词 -->/)
    assert.doesNotMatch(replySettingsSource, /<!-- 循环提示词 -->/)
    assert.match(settingsTabSource, /Goal 与继续回复/)
    assert.doesNotMatch(mainLayoutSource, /import GoalTab/)
    assert.doesNotMatch(mainLayoutSource, /name="goal"/)
  })
})
