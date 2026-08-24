import { invoke } from '@tauri-apps/api/core'

/**
 * 音频管理组合式函数
 */
export function useAudioManager() {
  /**
   * 播放音频文件
   */
  async function playAudio(audioName: string): Promise<void> {
    try {
      await invoke('play_audio', { audioName })
    }
    catch (error) {
      console.error('播放音频失败:', error)
      throw error
    }
  }

  /**
   * 停止音频播放
   */
  async function stopAudio(): Promise<void> {
    try {
      await invoke('stop_audio')
    }
    catch (error) {
      console.error('停止音频失败:', error)
      throw error
    }
  }

  /**
   * 获取可用的音频文件列表
   */
  async function getAvailableAudioFiles(): Promise<string[]> {
    try {
      return await invoke('get_available_audio_files')
    }
    catch (error) {
      console.error('获取音频文件列表失败:', error)
      return []
    }
  }

  /**
   * 测试音频播放
   */
  async function testAudio(audioName: string): Promise<void> {
    try {
      await playAudio(audioName)
      // 可以添加一些测试逻辑，比如播放一小段时间后停止
      setTimeout(async () => {
        try {
          await stopAudio()
        }
        catch (error) {
          console.warn('停止测试音频失败:', error)
        }
      }, 2000) // 2秒后停止
    }
    catch (error) {
      console.error('测试音频失败:', error)
      throw error
    }
  }

  /**
   * 处理音频测试错误
   */
  function handleTestAudioError(error: any) {
    console.error('音频测试错误:', error)
    // 这里可以显示错误提示给用户
  }

  return {
    playAudio,
    stopAudio,
    getAvailableAudioFiles,
    testAudio,
    handleTestAudioError,
  }
}
