export type Locale = 'en' | 'zh-CN';

export type LocaleOption = {
  value: Locale;
  label: string;
};

export const localeOptions: LocaleOption[] = [
  { value: 'en', label: 'English' },
  { value: 'zh-CN', label: '简体中文' },
];

export const defaultLocale: Locale = 'en';

const enMessages = {
  'nav.chat': 'Chat',
  'nav.canvas': 'Canvas',
  'nav.notifications': 'Notifications',
  'nav.settings': 'Settings',

  'settings.sidebar.profile': 'Profile',
  'settings.sidebar.workspace': 'Workspace',
  'settings.sidebar.notifications': 'Notifications',
  'settings.sidebar.debug': 'Debug',
  'settings.sidebar.help': 'Help',

  'settings.workspace.title': 'Workspace Settings',
  'settings.workspace.description':
    'Configure your technical environment, archival logic, and privacy protocols for the current atelier.',

  'settings.section.identity': 'Identity',
  'settings.field.workspaceName': 'Workspace Name',
  'settings.field.workspaceNameHint':
    'This name will be visible to all members of the workspace.',

  'settings.section.appearance': 'Appearance',
  'settings.field.visualMode': 'Visual Mode',
  'settings.field.visualModeHint': 'Select your preferred interface theme.',
  'settings.theme.system': 'System',
  'settings.theme.darkMode': 'Dark Mode',

  'settings.section.dataIntegrity': 'Data Integrity',
  'settings.field.archiveFrequency': 'Project Archive Frequency',
  'settings.field.archiveFrequencyHint':
    'Define the threshold for inactive object distillation.',
  'settings.field.archiveFrequencyValue': '30 Days',
  'settings.field.archiveFrequencyMin': '7 Days',
  'settings.field.archiveFrequencyMax': '90 Days',

  'settings.section.security': 'Security',
  'settings.field.searchVisibility': 'Global Search Visibility',
  'settings.field.searchVisibilityHint':
    'Allow objects to be indexed for cross-workspace discovery.',

  'settings.section.language': 'Localization',
  'settings.section.agentCapacity': 'Agent Capacity',
  'settings.field.language': 'Interface Language',
  'settings.field.languageHint': 'Choose your display language. Changes apply immediately.',
  'settings.field.languageActive': 'Active',
  'settings.field.maxAgentConcurrency': 'Max Agent Concurrency',
  'settings.field.maxAgentConcurrencyHint':
    'Controls how many agents can run at the same time. Extra work waits until capacity is available.',
  'settings.field.maxAgentConcurrencyInputLabel': 'Requested limit',
  'settings.field.maxAgentConcurrencyNormalizationHint':
    'Rust validates and normalizes the saved value, then this field refreshes to the persisted result.',
  'settings.field.maxAgentConcurrencyPending': 'Loading',
  'settings.field.maxAgentConcurrencyLoading': 'Loading saved agent capacity...',
  'settings.field.maxAgentConcurrencyLoaded': 'Saved agent capacity loaded.',
  'settings.field.maxAgentConcurrencySaving': 'Saving agent capacity...',
  'settings.field.maxAgentConcurrencySaved': 'Agent capacity saved.',
  'settings.field.maxAgentConcurrencyReset': 'Restored the saved agent capacity value.',
  'settings.field.maxAgentConcurrencyInvalid': 'Enter a whole number before saving.',
  'settings.field.maxAgentConcurrencySaveButton': 'Save Capacity',
  'settings.field.maxAgentConcurrencySavingButton': 'Saving...',

  'settings.action.discard': 'Discard',
  'settings.action.save': 'Save Changes',

  'settings.debug.sectionLabel': 'Settings / Debug',
  'settings.debug.back': 'Return',
  'settings.debug.title': 'Debug Workspace',
  'settings.debug.description':
    'This is the first migration step of the Tauri-to-Rust bridge. Commands below call the same backend surface used by apps/desktop.',
  'settings.debug.commands.title': 'Quick Commands',
  'settings.debug.commands.badge': 'Bridge',
  'settings.debug.action.createRun': 'Create Demo Run',
  'settings.debug.action.createSession': 'Create Demo Session',
  'settings.debug.action.listSessions': 'List Sessions',
  'settings.debug.output.title': 'Command Output',
  'settings.debug.output.default': 'No command executed yet.',
  'settings.debug.output.running': 'Running',
  'settings.debug.output.errorPrefix': 'Error: ',
  'settings.debug.output.bridgeMissing':
    'Tauri bridge is unavailable. Open this screen from the Tauri app instead of plain Vite browser mode.',

  'session.menu.rename': 'Rename',
  'session.menu.pin': 'Pin to top',
  'session.menu.unpin': 'Unpin',
  'session.menu.delete': 'Delete',
  'session.menu.actions': 'Session actions',
  'session.dialog.rename.badge': 'Session',
  'session.dialog.rename.title': 'Rename Session',
  'session.dialog.rename.description': 'Leave the field empty to restore automatic naming.',
  'session.dialog.rename.field': 'Session name',
  'session.dialog.rename.placeholder': 'Enter a custom title',
  'session.dialog.delete.badge': 'Danger',
  'session.dialog.delete.title': 'Delete Session?',
  'session.dialog.delete.descriptionPrefix': 'This will permanently remove',
  'session.dialog.delete.descriptionSuffix': 'and its timeline. This action cannot be undone.',
  'common.cancel': 'Cancel',
  'common.save': 'Save',
} as const;

export type MessageKey = keyof typeof enMessages;

const zhCnMessages: Record<MessageKey, string> = {
  'nav.chat': '对话',
  'nav.canvas': '画布',
  'nav.notifications': '通知',
  'nav.settings': '设置',

  'settings.sidebar.profile': '个人资料',
  'settings.sidebar.workspace': '工作区',
  'settings.sidebar.notifications': '通知',
  'settings.sidebar.debug': '调试',
  'settings.sidebar.help': '帮助',

  'settings.workspace.title': '工作区设置',
  'settings.workspace.description': '配置当前工作区的技术环境、归档策略与隐私协议。',

  'settings.section.identity': '标识',
  'settings.field.workspaceName': '工作区名称',
  'settings.field.workspaceNameHint': '此名称将对工作区所有成员可见。',

  'settings.section.appearance': '外观',
  'settings.field.visualMode': '界面模式',
  'settings.field.visualModeHint': '选择你偏好的界面主题。',
  'settings.theme.system': '跟随系统',
  'settings.theme.darkMode': '深色模式',

  'settings.section.dataIntegrity': '数据完整性',
  'settings.field.archiveFrequency': '项目归档周期',
  'settings.field.archiveFrequencyHint': '定义非活跃对象被归档蒸馏的时间阈值。',
  'settings.field.archiveFrequencyValue': '30 天',
  'settings.field.archiveFrequencyMin': '7 天',
  'settings.field.archiveFrequencyMax': '90 天',

  'settings.section.security': '安全',
  'settings.field.searchVisibility': '全局搜索可见性',
  'settings.field.searchVisibilityHint': '允许对象被索引以支持跨工作区发现。',

  'settings.section.language': '本地化',
  'settings.section.agentCapacity': 'Agent 容量',
  'settings.field.language': '界面语言',
  'settings.field.languageHint': '选择显示语言，切换将立即生效。',
  'settings.field.languageActive': '当前',
  'settings.field.maxAgentConcurrency': '最大 Agent 并发数',
  'settings.field.maxAgentConcurrencyHint': '控制可同时运行的 Agent 数量。超出的工作会等待直到有可用容量。',
  'settings.field.maxAgentConcurrencyInputLabel': '请求值',
  'settings.field.maxAgentConcurrencyNormalizationHint': '保存时由 Rust 负责校验和归一化，然后此字段会刷新为最终持久化的结果。',
  'settings.field.maxAgentConcurrencyPending': '加载中',
  'settings.field.maxAgentConcurrencyLoading': '正在加载已保存的 Agent 容量...',
  'settings.field.maxAgentConcurrencyLoaded': '已加载保存的 Agent 容量。',
  'settings.field.maxAgentConcurrencySaving': '正在保存 Agent 容量...',
  'settings.field.maxAgentConcurrencySaved': 'Agent 容量已保存。',
  'settings.field.maxAgentConcurrencyReset': '已恢复为保存的 Agent 容量值。',
  'settings.field.maxAgentConcurrencyInvalid': '请先输入整数再保存。',
  'settings.field.maxAgentConcurrencySaveButton': '保存容量',
  'settings.field.maxAgentConcurrencySavingButton': '保存中...',

  'settings.action.discard': '放弃',
  'settings.action.save': '保存更改',

  'settings.debug.sectionLabel': '设置 / 调试',
  'settings.debug.back': '返回',
  'settings.debug.title': '调试工作区',
  'settings.debug.description': '这是 Tauri 空壳接入 Rust 的第一步迁移。下面按钮调用与 apps/desktop 一致的后端命令。',
  'settings.debug.commands.title': '快捷命令',
  'settings.debug.commands.badge': '桥接',
  'settings.debug.action.createRun': '创建示例 Run',
  'settings.debug.action.createSession': '创建示例 Session',
  'settings.debug.action.listSessions': '列出 Session',
  'settings.debug.output.title': '命令输出',
  'settings.debug.output.default': '尚未执行任何命令。',
  'settings.debug.output.running': '执行中',
  'settings.debug.output.errorPrefix': '错误：',
  'settings.debug.output.bridgeMissing': '当前没有 Tauri 桥接，请在 Tauri 应用中打开此页面，而不是纯 Vite 浏览器模式。',

  'session.menu.rename': '重命名',
  'session.menu.pin': '置顶',
  'session.menu.unpin': '取消置顶',
  'session.menu.delete': '删除',
  'session.menu.actions': 'Session 操作',
  'session.dialog.rename.badge': 'Session',
  'session.dialog.rename.title': '重命名 Session',
  'session.dialog.rename.description': '如果留空，将恢复自动命名。',
  'session.dialog.rename.field': 'Session 名称',
  'session.dialog.rename.placeholder': '输入自定义标题',
  'session.dialog.delete.badge': '危险操作',
  'session.dialog.delete.title': '删除 Session？',
  'session.dialog.delete.descriptionPrefix': '这会永久删除',
  'session.dialog.delete.descriptionSuffix': '及其时间线，此操作无法撤销。',
  'common.cancel': '取消',
  'common.save': '保存',
};

export const messages: Record<Locale, Record<MessageKey, string>> = {
  en: enMessages,
  'zh-CN': zhCnMessages,
};

export function isLocale(value: string): value is Locale {
  return value === 'en' || value === 'zh-CN';
}
