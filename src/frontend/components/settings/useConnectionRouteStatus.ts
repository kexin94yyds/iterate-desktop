export type RouteLaneStatus = 'ok' | 'degraded' | 'maintenance' | 'available' | 'down' | 'unknown'

export interface ConnectionStatusSnapshot {
  diagnosis?: { code?: string }
  local_origin?: { healthy?: boolean }
  public_tunnel?: { healthy?: boolean, health_source?: string }
  root_tunnel?: {
    derived?: {
      ha_degraded?: boolean
      ha_ready?: boolean
      backoff_active?: boolean
      ha_active?: boolean
      backoff_remaining_secs?: number
      structural_block?: boolean
      edge_7844_suspected?: boolean
      tunnel_health_class?: string
      last_skip_reason?: string
    }
    status?: {
      diagnosis_code?: string
      structural_block?: boolean
      ha_connection_count?: number
      expected_ha_connections?: number
      backoff_remaining_secs?: number
    }
  }
}

export interface RouteLaneView {
  status: RouteLaneStatus
  title: string
  detail: string
  dotClass: string
}

export interface ConnectionRouteView {
  localBridge: RouteLaneView
  tailscale: RouteLaneView
  publicRoute: RouteLaneView
  showPublicMaintenanceHint: boolean
  maintenanceHint: string
}

function dotClassFor(status: RouteLaneStatus) {
  switch (status) {
    case 'ok':
      return 'bg-success'
    case 'available':
      return 'bg-success'
    case 'degraded':
    case 'maintenance':
      return 'bg-warning'
    case 'down':
      return 'bg-error'
    default:
      return 'bg-gray-400'
  }
}

function lane(status: RouteLaneStatus, title: string, detail: string): RouteLaneView {
  return { status, title, detail, dotClass: dotClassFor(status) }
}

function diagnosisCode(snapshot: ConnectionStatusSnapshot | null) {
  return snapshot?.root_tunnel?.status?.diagnosis_code
    || snapshot?.diagnosis?.code
    || ''
}

function publicHealthy(snapshot: ConnectionStatusSnapshot | null) {
  return snapshot?.public_tunnel?.healthy === true
}

function rootDerived(snapshot: ConnectionStatusSnapshot | null) {
  return snapshot?.root_tunnel?.derived
}

export function buildConnectionRouteView(input: {
  connectionStatus: ConnectionStatusSnapshot | null
  localBridgeHealthy?: boolean
  tailscaleClientOnline: boolean
  tailscaleCandidateAvailable: boolean
}): ConnectionRouteView {
  const { connectionStatus, tailscaleClientOnline, tailscaleCandidateAvailable } = input
  const localOk = connectionStatus?.local_origin?.healthy ?? input.localBridgeHealthy ?? false
  const publicOk = publicHealthy(connectionStatus)
  const derived = rootDerived(connectionStatus)
  const code = diagnosisCode(connectionStatus)
  const haCount = connectionStatus?.root_tunnel?.status?.ha_connection_count
  const expectedHa = connectionStatus?.root_tunnel?.status?.expected_ha_connections
  const haLabel = typeof haCount === 'number' && typeof expectedHa === 'number'
    ? `HA ${haCount}/${expectedHa}`
    : null

  const localBridge = localOk
    ? lane('ok', '本地 bridge', '8080 正常，Mac 本机服务可用')
    : lane('down', '本地 bridge', code === 'local_origin_down' ? '8080 不可达' : '8080 不可达，请先恢复本地服务')

  let tailscale: RouteLaneView
  if (tailscaleClientOnline)
    tailscale = lane('ok', 'Tailscale 主路', 'iPhone 当前经 Tailscale 在线')
  else if (tailscaleCandidateAvailable && localOk)
    tailscale = lane('available', 'Tailscale 主路', '配对含 Tailscale 候选，可优先使用')
  else if (tailscaleCandidateAvailable)
    tailscale = lane('unknown', 'Tailscale 主路', '已配置 Tailscale，等待本地 bridge 恢复')
  else
    tailscale = lane('unknown', 'Tailscale 主路', '未检测到 Tailscale 候选或在线设备')

  let publicRoute: RouteLaneView
  if (publicOk) {
    const source = connectionStatus?.public_tunnel?.health_source
    publicRoute = lane(
      'ok',
      '公网备路',
      source === 'root_tunnel_ha' && haLabel ? `公网可用（${haLabel}）` : '公网域名可达',
    )
  }
  else if (!localOk) {
    publicRoute = lane('unknown', '公网备路', '等待本地 bridge 恢复后再检测')
  }
  else if (
    connectionStatus?.root_tunnel?.derived?.structural_block
    || connectionStatus?.root_tunnel?.status?.structural_block
    || code === 'blocked_by_edge_connectivity'
  ) {
    publicRoute = lane('maintenance', '公网备路', 'Cloudflare edge 暂不可达，自动维护中')
  }
  else if (derived?.backoff_active) {
    const remain = derived.backoff_remaining_secs
      ?? connectionStatus?.root_tunnel?.status?.backoff_remaining_secs
    publicRoute = lane(
      'degraded',
      '公网备路',
      typeof remain === 'number' && remain > 0
        ? `隧道恢复冷却中（约 ${remain}s）${haLabel ? ` · ${haLabel}` : ''}`
        : `隧道恢复冷却中${haLabel ? ` · ${haLabel}` : ''}`,
    )
  }
  else if (derived?.ha_degraded) {
    publicRoute = lane('degraded', '公网备路', haLabel ? `连接降级 · ${haLabel}` : 'Cloudflare HA 未就绪')
  }
  else {
    publicRoute = lane('degraded', '公网备路', '公网探针失败，备路维护中')
  }

  const tailscaleUsable = tailscaleClientOnline || (tailscaleCandidateAvailable && localOk)
  const showPublicMaintenanceHint = !publicOk && localOk && tailscaleUsable

  return {
    localBridge,
    tailscale,
    publicRoute,
    showPublicMaintenanceHint,
    maintenanceHint: '公网备路维护中，请继续用 Tailscale',
  }
}
