import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

function source(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), 'utf8')
}

test('Windows registry liveness check does not spawn tasklist', () => {
  const registry = source('src/rust/ui/window_registry.rs')
  assert.doesNotMatch(registry, /Command::new\("tasklist"\)/)
  assert.match(registry, /OpenProcess\(PROCESS_QUERY_LIMITED_INFORMATION/)
})

test('bridge health probe uses reqwest and does not spawn curl', () => {
  const setup = source('src/rust/app/setup.rs')
  assert.doesNotMatch(setup, /command_stdout\("curl"/)
  assert.match(setup, /reqwest::blocking::Client::builder/)
})

test('main window is shown before background setup starts', () => {
  const builder = source('src/rust/app/builder.rs')
  assert.doesNotMatch(builder, /async_runtime::block_on/)
  const showIndex = builder.indexOf('window.show()')
  const setupIndex = builder.lastIndexOf('start_application_setup(app_handle)')
  assert.ok(showIndex >= 0 && setupIndex > showIndex)
})

test('optional speech startup failure cannot block the main application', () => {
  const app = source('src/frontend/App.vue')
  assert.match(
    app,
    /try\s*{\s*await speechRuntimeHost\.initialize\(\)\s*}\s*catch \(error\)\s*{[\s\S]*?speechRuntimeDegraded/,
  )
  assert.ok(
    app.indexOf('const activationGateRequired = await requiresActivationGate()')
    > app.indexOf('onMounted:speechRuntimeDegraded'),
    'authorization and main-window startup must continue after optional speech initialization fails',
  )
})

test('window registry cleanup runs off the UI thread at low frequency', () => {
  const events = source('src/rust/ui/window_events.rs')
  const builder = source('src/rust/app/builder.rs')
  assert.match(events, /WINDOW_REGISTRY_CLEANUP_INTERVAL[^;]*Duration::from_secs\(5 \* 60\)/s)
  assert.match(events, /start_window_registry_cleanup_task[\s\S]*spawn_blocking[\s\S]*get_all_instances/)
  assert.match(builder, /start_window_registry_cleanup_task\(\)/)
})

test('close flow exits without recursively closing the window', () => {
  const exit = source('src/rust/ui/exit.rs')
  const lifecycle = source('src/rust/app/windows_lifecycle.rs')
  assert.match(exit, /exit_in_progress\.swap/)
  assert.doesNotMatch(exit, /window\.close\(\)/)
  assert.match(exit, /window\.hide\(\)/)
  assert.match(exit, /request_global_shutdown\(\)/)
  assert.match(lifecycle, /CreateEventW/)
  assert.match(lifecycle, /QueryFullProcessImageNameW/)
  assert.match(lifecycle, /GetProcessTimes/)
  assert.match(lifecycle, /OpenProcessToken/)
  assert.match(lifecycle, /GetTokenInformation/)
  assert.match(lifecycle, /TerminateProcess/)
  assert.doesNotMatch(lifecycle, /taskkill|tasklist|Stop-Process|Command::new/)
})

test('manual close blocks automatic zhi startup until a shortcut launch clears it', () => {
  const lifecycle = source('src/rust/app/windows_lifecycle.rs')
  const main = source('src/rust/main.rs')
  const mcpServer = source('src/bin/mcp-server.rs')
  assert.match(lifecycle, /args\.len\(\) != 1/)
  assert.match(lifecycle, /remove_file\(manual_stop_path\(\)\)/)
  assert.match(main, /activate_manual_launch_if_requested/)
  assert.match(mcpServer, /is_manually_stopped\(\)/)
  assert.match(mcpServer, /MANUALLY_STOPPED_MESSAGE/)
})

test('explicit conversation end is exact and is normalized at the response boundary', () => {
  const command = source('src/rust/conversation/end_command.rs')
  const cli = source('src/rust/app/cli.rs')
  const interaction = source('src/rust/mcp/tools/interaction/mcp.rs')
  const popup = source('src/frontend/components/popup/PopupInput.vue')
  const frontendCommand = source('src/frontend/utils/conversationEndCommand.ts')
  assert.match(command, /"结束对话"/)
  assert.match(command, /"\/end"/)
  assert.match(command, /eq_ignore_ascii_case/)
  assert.match(command, /selected_options[\s\S]*?is_explicit_conversation_end/)
  assert.match(cli, /keep_going: !interaction_ended/)
  assert.match(cli, /EXPLICIT_CONVERSATION_END_SOURCE/)
  assert.match(cli, /POPUP_CLOSED_SOURCE/)
  assert.match(interaction, /继续对话: false/)
  assert.match(popup, /输入“结束对话”或 \/end 可结束本次交互/)
  assert.match(frontendCommand, /isExplicitConversationEndInput/)
  assert.ok(
    popup.indexOf('isExplicitConversationEndInput(clipboardText)')
    < popup.indexOf('extractClipboardPaths(clipboardText)'),
    'explicit end commands must bypass clipboard path attachment handling',
  )
  assert.match(
    popup,
    /!isExplicitConversationEndInput\(userInput\.value\)[\s\S]*?generateConditionalContent\(\)/,
    'explicit end commands must reach the Rust response boundary without appended context',
  )
})

test('popup close ends only the current interaction while the native titlebar still exits', () => {
  const header = source('src/frontend/components/popup/PopupHeader.vue')
  const content = source('src/frontend/components/AppContent.vue')
  const app = source('src/frontend/App.vue')
  const handler = source('src/frontend/composables/useMcpHandler.ts')
  const windowEvents = source('src/rust/ui/window_events.rs')
  assert.match(header, /closeCurrentDialog/)
  assert.match(header, /结束当前对话（iterate 继续运行）/)
  assert.match(content, /mcpCloseCurrentDialog/)
  assert.match(app, /mcp-close-current-dialog/)
  assert.match(handler, /handleMcpCloseCurrentDialog/)
  assert.match(handler, /source: 'popup_closed'/)
  assert.match(handler, /resolvingRequestIds/)
  assert.match(windowEvents, /handle_system_exit_request[\s\S]*?true/)
})

test('Windows bundle uses a current-user NSIS installer', () => {
  const config = JSON.parse(source('tauri.conf.json'))
  assert.equal(config.bundle.windows.nsis.installMode, 'currentUser')
})
