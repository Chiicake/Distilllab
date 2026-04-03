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
  'settings.field.language': 'Interface Language',
  'settings.field.languageHint': 'Choose your display language. Changes apply immediately.',
  'settings.field.languageActive': 'Active',

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
  'settings.field.language': '界面语言',
  'settings.field.languageHint': '选择显示语言，切换将立即生效。',
  'settings.field.languageActive': '当前',

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
};

export const messages: Record<Locale, Record<MessageKey, string>> = {
  en: enMessages,
  'zh-CN': zhCnMessages,
};

export function isLocale(value: string): value is Locale {
  return value === 'en' || value === 'zh-CN';
}
