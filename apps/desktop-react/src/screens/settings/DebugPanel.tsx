import { useCallback, useEffect, useMemo, useState } from 'react';

import { useI18n } from '../../i18n/I18nProvider';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

type SessionSelectorOption = {
  sessionId: string;
  title: string;
  status: string;
  label: string;
};

type LlmProviderOptions = {
  baseURL?: string;
  baseUrl?: string;
  apiKey?: string;
};

type LlmProviderEntry = {
  name?: string;
  npm?: string;
  options?: LlmProviderOptions;
  models?: Record<string, { name?: string }>;
};

type LlmConfig = {
  provider?: Record<string, LlmProviderEntry>;
  distilllab?: {
    currentProvider?: string;
    currentModel?: string;
  };
};

type ConfigFormState = {
  currentProvider: string;
  currentModel: string;
  providerName: string;
  providerNpm: string;
  baseUrl: string;
  apiKey: string;
  providerJson: string;
  importPath: string;
};

type TimelineFormState = {
  sessionId: string;
  message: string;
  attachments: string;
};

type QuickCommandAction = {
  command:
    | 'create_demo_run'
    | 'create_demo_session'
    | 'create_demo_source'
    | 'chunk_demo_source'
    | 'extract_demo_work_items'
    | 'group_demo_project'
    | 'build_demo_assets'
    | 'list_runs'
    | 'list_sessions'
    | 'list_sources'
    | 'list_work_items'
    | 'list_projects'
    | 'list_assets'
    | 'list_chunks_for_source';
};

const quickActions: QuickCommandAction[] = [
  { command: 'create_demo_run' },
  { command: 'create_demo_session' },
  { command: 'create_demo_source' },
  { command: 'chunk_demo_source' },
  { command: 'extract_demo_work_items' },
  { command: 'group_demo_project' },
  { command: 'build_demo_assets' },
  { command: 'list_runs' },
  { command: 'list_sessions' },
  { command: 'list_sources' },
  { command: 'list_work_items' },
  { command: 'list_projects' },
  { command: 'list_assets' },
];

const defaultConfigFormState: ConfigFormState = {
  currentProvider: '',
  currentModel: '',
  providerName: '',
  providerNpm: '@ai-sdk/openai-compatible',
  baseUrl: '',
  apiKey: '',
  providerJson: '',
  importPath: '',
};

const defaultTimelineFormState: TimelineFormState = {
  sessionId: '',
  message: '',
  attachments: '',
};

type DebugText = {
  statusLoadingConfig: string;
  statusLoadingTimeline: string;
  statusSavingConfig: string;
  statusCreatingProvider: string;
  statusDeletingProvider: string;
  statusImportingProviders: string;
  statusTestingProvider: string;
  statusCreatingSession: string;
  statusSendingMessage: string;
  statusPreviewingIntake: string;
  validationEnterSessionId: string;
  validationEnterSourceId: string;
  validationProviderModelRequired: string;
  validationSessionParseFailed: string;
  validationSessionAndMessageRequired: string;
  promptNewProviderId: string;
  confirmDeleteProvider: string;
  quickActionsTitle: string;
  quickActionsBadge: string;
  quickActionsDescription: string;
  sourceIdPlaceholder: string;
  actionListChunks: string;
  configTitle: string;
  configDescription: string;
  configCurrentProvider: string;
  configCurrentModel: string;
  configProviderName: string;
  configProviderNpm: string;
  configBaseUrl: string;
  configApiKey: string;
  configProviderJson: string;
  configImportPath: string;
  configImportPathPlaceholder: string;
  actionLoadConfig: string;
  actionNewProvider: string;
  actionSaveProvider: string;
  actionDeleteProvider: string;
  actionImportProviders: string;
  actionTestProvider: string;
  configOutputTitle: string;
  timelineTitle: string;
  timelineDescription: string;
  timelineExistingSessions: string;
  timelineSelectSession: string;
  timelineSessionId: string;
  timelineSessionIdPlaceholder: string;
  timelineMessage: string;
  timelineMessagePlaceholder: string;
  timelineAttachments: string;
  timelineAttachmentsPlaceholder: string;
  actionCreateAndUseSession: string;
  actionRefreshSessions: string;
  actionSendMessage: string;
  actionRefreshTimeline: string;
  timelineOutputTitle: string;
  outputGeneralTitle: string;
};

const debugTextEn: DebugText = {
  statusLoadingConfig: 'Loading config...',
  statusLoadingTimeline: 'Loading timeline...',
  statusSavingConfig: 'Saving config...',
  statusCreatingProvider: 'Creating provider...',
  statusDeletingProvider: 'Deleting provider...',
  statusImportingProviders: 'Importing providers...',
  statusTestingProvider: 'Testing provider...',
  statusCreatingSession: 'Creating session...',
  statusSendingMessage: 'Sending message...',
  statusPreviewingIntake: 'Previewing session intake...',
  validationEnterSessionId: 'Enter a session ID first.',
  validationEnterSourceId: 'Enter a source ID first.',
  validationProviderModelRequired: 'Current provider and model are required.',
  validationSessionParseFailed: 'Could not parse created session ID from response.',
  validationSessionAndMessageRequired: 'Session ID and message are required.',
  promptNewProviderId: 'Enter a new provider ID:',
  confirmDeleteProvider: 'Delete provider',
  quickActionsTitle: 'Quick Commands',
  quickActionsBadge: 'Bridge',
  quickActionsDescription: 'Direct command shortcuts from the migrated desktop debug surface.',
  sourceIdPlaceholder: 'source-... (for chunk lookup)',
  actionListChunks: 'List Chunks',
  configTitle: 'LLM Config Bar',
  configDescription:
    'Session and debug LLM calls read from the current provider/model configured here.',
  configCurrentProvider: 'Current Provider',
  configCurrentModel: 'Current Model',
  configProviderName: 'Provider Name',
  configProviderNpm: 'Provider NPM',
  configBaseUrl: 'Base URL',
  configApiKey: 'API Key',
  configProviderJson: 'Advanced Provider JSON',
  configImportPath: 'OpenCode Config Path',
  configImportPathPlaceholder: 'optional, defaults to ~/.config/opencode/opencode.json',
  actionLoadConfig: 'Load Config',
  actionNewProvider: 'New Provider',
  actionSaveProvider: 'Save Provider',
  actionDeleteProvider: 'Delete Provider',
  actionImportProviders: 'Import OpenCode',
  actionTestProvider: 'Test Provider',
  configOutputTitle: 'Config Output',
  timelineTitle: 'Session Timeline',
  timelineDescription:
    'Create/use sessions, send messages, and inspect timeline output through Rust commands.',
  timelineExistingSessions: 'Existing Sessions',
  timelineSelectSession: 'Select an existing session...',
  timelineSessionId: 'Session ID',
  timelineSessionIdPlaceholder: 'session-...',
  timelineMessage: 'Session Message',
  timelineMessagePlaceholder: 'Type a user message...',
  timelineAttachments: 'Attachment Paths',
  timelineAttachmentsPlaceholder: '/path/to/file\n/path/to/file2',
  actionCreateAndUseSession: 'Create & Use Session',
  actionRefreshSessions: 'Refresh Sessions',
  actionSendMessage: 'Send Message',
  actionRefreshTimeline: 'Refresh Timeline',
  timelineOutputTitle: 'Timeline Output',
  outputGeneralTitle: 'General Output',
};

const debugTextZhCn: DebugText = {
  statusLoadingConfig: '正在加载配置...',
  statusLoadingTimeline: '正在加载时间线...',
  statusSavingConfig: '正在保存配置...',
  statusCreatingProvider: '正在创建 Provider...',
  statusDeletingProvider: '正在删除 Provider...',
  statusImportingProviders: '正在导入 Providers...',
  statusTestingProvider: '正在测试 Provider...',
  statusCreatingSession: '正在创建 Session...',
  statusSendingMessage: '正在发送消息...',
  statusPreviewingIntake: '正在预览会话 Intake...',
  validationEnterSessionId: '请先输入 Session ID。',
  validationEnterSourceId: '请先输入 Source ID。',
  validationProviderModelRequired: '当前 Provider 和 Model 不能为空。',
  validationSessionParseFailed: '无法从返回结果中解析新建 Session ID。',
  validationSessionAndMessageRequired: 'Session ID 和消息不能为空。',
  promptNewProviderId: '输入新的 Provider ID：',
  confirmDeleteProvider: '确认删除 Provider',
  quickActionsTitle: '快捷命令',
  quickActionsBadge: '桥接',
  quickActionsDescription: '来自迁移后桌面调试面的直接命令快捷入口。',
  sourceIdPlaceholder: 'source-...（用于 chunk 查询）',
  actionListChunks: '列出 Chunks',
  configTitle: 'LLM 配置栏',
  configDescription: '会话与调试相关 LLM 调用都读取这里配置的当前 Provider/Model。',
  configCurrentProvider: '当前 Provider',
  configCurrentModel: '当前 Model',
  configProviderName: 'Provider 名称',
  configProviderNpm: 'Provider NPM',
  configBaseUrl: 'Base URL',
  configApiKey: 'API Key',
  configProviderJson: '高级 Provider JSON',
  configImportPath: 'OpenCode 配置路径',
  configImportPathPlaceholder: '可选，默认 ~/.config/opencode/opencode.json',
  actionLoadConfig: '加载配置',
  actionNewProvider: '新建 Provider',
  actionSaveProvider: '保存 Provider',
  actionDeleteProvider: '删除 Provider',
  actionImportProviders: '导入 OpenCode',
  actionTestProvider: '测试 Provider',
  configOutputTitle: '配置输出',
  timelineTitle: '会话时间线',
  timelineDescription: '通过 Rust 命令创建/选择会话、发送消息并查看时间线输出。',
  timelineExistingSessions: '现有 Sessions',
  timelineSelectSession: '选择已有 Session...',
  timelineSessionId: 'Session ID',
  timelineSessionIdPlaceholder: 'session-...',
  timelineMessage: '会话消息',
  timelineMessagePlaceholder: '输入一条用户消息...',
  timelineAttachments: '附件路径',
  timelineAttachmentsPlaceholder: '/path/to/file\n/path/to/file2',
  actionCreateAndUseSession: '创建并使用 Session',
  actionRefreshSessions: '刷新 Sessions',
  actionSendMessage: '发送消息',
  actionRefreshTimeline: '刷新时间线',
  timelineOutputTitle: '时间线输出',
  outputGeneralTitle: '通用输出',
};

function resolveDebugText(locale: string): DebugText {
  return locale === 'zh-CN' ? debugTextZhCn : debugTextEn;
}

function quickActionLabel(
  command: QuickCommandAction['command'],
  t: (key: 'settings.debug.action.createRun' | 'settings.debug.action.createSession' | 'settings.debug.action.listSessions') => string,
): string {
  switch (command) {
    case 'create_demo_run':
      return t('settings.debug.action.createRun');
    case 'create_demo_session':
      return t('settings.debug.action.createSession');
    case 'create_demo_source':
      return 'Create Demo Source';
    case 'chunk_demo_source':
      return 'Chunk Demo Source';
    case 'extract_demo_work_items':
      return 'Extract Demo Work Items';
    case 'group_demo_project':
      return 'Group Demo Project';
    case 'build_demo_assets':
      return 'Build Demo Assets';
    case 'list_runs':
      return 'List Runs';
    case 'list_sessions':
      return t('settings.debug.action.listSessions');
    case 'list_sources':
      return 'List Sources';
    case 'list_work_items':
      return 'List Work Items';
    case 'list_projects':
      return 'List Projects';
    case 'list_assets':
      return 'List Assets';
    case 'list_chunks_for_source':
      return 'List Chunks';
  }
}

function resolveTauriInvoke(): TauriInvoke | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const tauriInternals = (window as Window & {
    __TAURI_INTERNALS__?: { invoke?: TauriInvoke };
  }).__TAURI_INTERNALS__;

  return tauriInternals?.invoke ?? null;
}

function parseErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

export default function DebugPanel() {
  const { locale, t } = useI18n();
  const text = useMemo(() => resolveDebugText(locale), [locale]);
  const invoke = useMemo(resolveTauriInvoke, []);
  const bridgeMissingText = t('settings.debug.output.bridgeMissing');
  const [generalOutput, setGeneralOutput] = useState<string>(t('settings.debug.output.default'));
  const [configOutput, setConfigOutput] = useState<string>(t('settings.debug.output.default'));
  const [timelineOutput, setTimelineOutput] = useState<string>(t('settings.debug.output.default'));

  const [sourceId, setSourceId] = useState('');

  const [configForm, setConfigForm] = useState<ConfigFormState>(defaultConfigFormState);
  const [providerOptions, setProviderOptions] = useState<string[]>([]);
  const [modelOptions, setModelOptions] = useState<string[]>([]);

  const [timelineForm, setTimelineForm] = useState<TimelineFormState>(defaultTimelineFormState);
  const [sessionOptions, setSessionOptions] = useState<SessionSelectorOption[]>([]);

  const [isRunningGeneral, setIsRunningGeneral] = useState(false);
  const [isRunningConfig, setIsRunningConfig] = useState(false);
  const [isRunningTimeline, setIsRunningTimeline] = useState(false);

  const setFormattedError = useCallback(
    (setOutput: (value: string) => void, error: unknown) => {
      setOutput(`${t('settings.debug.output.errorPrefix')}${parseErrorMessage(error)}`);
    },
    [t],
  );

  const parseLlmConfig = useCallback((rawJson: string): LlmConfig => {
    const value = JSON.parse(rawJson) as LlmConfig;
    return value;
  }, []);

  const hydrateConfigFormFromConfig = useCallback(
    (config: LlmConfig, preferredProviderId?: string, preferredModelId?: string) => {
      const providers = config.provider ?? {};
      const providerIds = Object.keys(providers);

      const resolvedProviderId =
        (preferredProviderId && providers[preferredProviderId] && preferredProviderId) ||
        (config.distilllab?.currentProvider && providers[config.distilllab.currentProvider]
          ? config.distilllab.currentProvider
          : undefined) ||
        providerIds[0] ||
        '';

      const provider = resolvedProviderId ? providers[resolvedProviderId] : undefined;
      const modelIds = Object.keys(provider?.models ?? {});

      const resolvedModelId =
        (preferredModelId && modelIds.includes(preferredModelId) && preferredModelId) ||
        (config.distilllab?.currentModel && modelIds.includes(config.distilllab.currentModel)
          ? config.distilllab.currentModel
          : undefined) ||
        modelIds[0] ||
        '';

      setProviderOptions(providerIds);
      setModelOptions(modelIds);
      setConfigForm((previous) => ({
        ...previous,
        currentProvider: resolvedProviderId,
        currentModel: resolvedModelId,
        providerName: provider?.name ?? '',
        providerNpm: provider?.npm ?? '@ai-sdk/openai-compatible',
        baseUrl: provider?.options?.baseURL ?? provider?.options?.baseUrl ?? '',
        apiKey: provider?.options?.apiKey ?? '',
        providerJson: provider ? JSON.stringify(provider, null, 2) : '',
      }));
    },
    [],
  );

  const loadConfigSummary = useCallback(async () => {
    if (!invoke) {
      setConfigOutput(bridgeMissingText);
      return;
    }

    setIsRunningConfig(true);
    setConfigOutput(text.statusLoadingConfig);

    try {
      const summary = await invoke<string>('load_llm_config_command');
      const rawJson = await invoke<string>('load_llm_config_json_command');
      const config = parseLlmConfig(rawJson);

      hydrateConfigFormFromConfig(config);
      setConfigOutput(summary);
    } catch (error) {
      setFormattedError(setConfigOutput, error);
    } finally {
      setIsRunningConfig(false);
    }
  }, [bridgeMissingText, hydrateConfigFormFromConfig, invoke, parseLlmConfig, setFormattedError, t]);

  const refreshSessionOptions = useCallback(
    async (preferredSessionId?: string) => {
      if (!invoke) {
        setTimelineOutput(bridgeMissingText);
        return;
      }

      try {
        const response = await invoke<string>('list_session_selector_options');
        const sessions = JSON.parse(response) as SessionSelectorOption[];
        setSessionOptions(sessions);

        setTimelineForm((previous) => {
          const candidateSessionId = preferredSessionId ?? previous.sessionId;
          const hasCandidate = sessions.some((session) => session.sessionId === candidateSessionId);
          return {
            ...previous,
            sessionId: hasCandidate ? candidateSessionId : '',
          };
        });
      } catch (error) {
        setFormattedError(setTimelineOutput, error);
      }
    },
    [bridgeMissingText, invoke, setFormattedError],
  );

  const refreshTimeline = useCallback(
    async (sessionId: string) => {
      if (!invoke) {
        setTimelineOutput(bridgeMissingText);
        return;
      }

      if (!sessionId.trim()) {
        setTimelineOutput(text.validationEnterSessionId);
        return;
      }

      setIsRunningTimeline(true);
      setTimelineOutput(text.statusLoadingTimeline);

      try {
        const response = await invoke<string>('list_session_messages_command', {
          sessionId: sessionId.trim(),
        });
        setTimelineOutput(response);
      } catch (error) {
        setFormattedError(setTimelineOutput, error);
      } finally {
        setIsRunningTimeline(false);
      }
    },
    [bridgeMissingText, invoke, setFormattedError, t],
  );

  const runGeneralCommand = useCallback(
    async (command: QuickCommandAction['command'], args?: Record<string, unknown>) => {
      if (!invoke) {
        setGeneralOutput(bridgeMissingText);
        return;
      }

      setIsRunningGeneral(true);
      setGeneralOutput(`${t('settings.debug.output.running')} ${command}...`);

      try {
        const response = await invoke<string>(command, args);
        setGeneralOutput(response);
      } catch (error) {
        setFormattedError(setGeneralOutput, error);
      } finally {
        setIsRunningGeneral(false);
      }
    },
    [bridgeMissingText, invoke, setFormattedError, t],
  );

  const handleRunQuickAction = useCallback(
    async (action: QuickCommandAction) => {
      await runGeneralCommand(action.command);
    },
    [runGeneralCommand],
  );

  const handleListChunks = useCallback(async () => {
    if (!sourceId.trim()) {
      setGeneralOutput(text.validationEnterSourceId);
      return;
    }

    await runGeneralCommand('list_chunks_for_source', {
      sourceId: sourceId.trim(),
    });
  }, [runGeneralCommand, sourceId, text.validationEnterSourceId]);

  const handleConfigProviderChange = useCallback(
    async (nextProviderId: string) => {
      setConfigForm((previous) => ({ ...previous, currentProvider: nextProviderId }));

      if (!invoke) {
        setConfigOutput(bridgeMissingText);
        return;
      }

      setIsRunningConfig(true);

      try {
        const rawJson = await invoke<string>('load_llm_config_json_command');
        const config = parseLlmConfig(rawJson);
        const models = Object.keys(config.provider?.[nextProviderId]?.models ?? {});
        const nextModelId = models[0] ?? '';

        if (nextProviderId && nextModelId) {
          const response = await invoke<string>('set_current_provider_model_command', {
            providerId: nextProviderId,
            modelId: nextModelId,
          });
          setConfigOutput(response);
        }

        hydrateConfigFormFromConfig(config, nextProviderId, nextModelId);
      } catch (error) {
        setFormattedError(setConfigOutput, error);
      } finally {
        setIsRunningConfig(false);
      }
    },
    [bridgeMissingText, hydrateConfigFormFromConfig, invoke, parseLlmConfig, setFormattedError],
  );

  const handleConfigModelChange = useCallback(
    async (nextModelId: string) => {
      setConfigForm((previous) => ({ ...previous, currentModel: nextModelId }));

      if (!invoke) {
        setConfigOutput(bridgeMissingText);
        return;
      }

      if (!configForm.currentProvider || !nextModelId) {
        return;
      }

      setIsRunningConfig(true);

      try {
        const response = await invoke<string>('set_current_provider_model_command', {
          providerId: configForm.currentProvider,
          modelId: nextModelId,
        });
        setConfigOutput(response);

        const rawJson = await invoke<string>('load_llm_config_json_command');
        const config = parseLlmConfig(rawJson);
        hydrateConfigFormFromConfig(config, configForm.currentProvider, nextModelId);
      } catch (error) {
        setFormattedError(setConfigOutput, error);
      } finally {
        setIsRunningConfig(false);
      }
    },
    [bridgeMissingText, configForm.currentProvider, hydrateConfigFormFromConfig, invoke, parseLlmConfig, setFormattedError],
  );

  const handleSaveCurrentProvider = useCallback(async () => {
    if (!invoke) {
      setConfigOutput(bridgeMissingText);
      return;
    }

    if (!configForm.currentProvider || !configForm.currentModel) {
      setConfigOutput(text.validationProviderModelRequired);
      return;
    }

    setIsRunningConfig(true);
    setConfigOutput(text.statusSavingConfig);

    try {
      const response = await invoke<string>('save_llm_config_command', {
        form: {
          currentProvider: configForm.currentProvider,
          currentModel: configForm.currentModel,
          providerName: configForm.providerName,
          providerNpm: configForm.providerNpm,
          baseUrl: configForm.baseUrl,
          apiKey: configForm.apiKey,
          rawProviderJson: configForm.providerJson,
        },
      });
      setConfigOutput(response);
    } catch (error) {
      setFormattedError(setConfigOutput, error);
    } finally {
      setIsRunningConfig(false);
    }
  }, [bridgeMissingText, configForm, invoke, setFormattedError, text.statusSavingConfig, text.validationProviderModelRequired]);

  const handleCreateProvider = useCallback(async () => {
    if (!invoke) {
      setConfigOutput(bridgeMissingText);
      return;
    }

    const providerId = window.prompt(text.promptNewProviderId, 'new-provider');
    if (!providerId || !providerId.trim()) {
      return;
    }

    setIsRunningConfig(true);
    setConfigOutput(text.statusCreatingProvider);

    try {
      const response = await invoke<string>('create_provider_command', {
        providerId: providerId.trim(),
      });
      setConfigOutput(response);
      await loadConfigSummary();
    } catch (error) {
      setFormattedError(setConfigOutput, error);
    } finally {
      setIsRunningConfig(false);
    }
  }, [
    bridgeMissingText,
    invoke,
    loadConfigSummary,
    setFormattedError,
    text.promptNewProviderId,
    text.statusCreatingProvider,
  ]);

  const handleDeleteProvider = useCallback(async () => {
    if (!invoke) {
      setConfigOutput(bridgeMissingText);
      return;
    }

    if (!configForm.currentProvider) {
      setConfigOutput(text.validationProviderModelRequired);
      return;
    }

    const confirmed = window.confirm(`${text.confirmDeleteProvider} ${configForm.currentProvider}?`);
    if (!confirmed) {
      return;
    }

    setIsRunningConfig(true);
    setConfigOutput(text.statusDeletingProvider);

    try {
      const response = await invoke<string>('delete_provider_command', {
        providerId: configForm.currentProvider,
      });
      setConfigOutput(response);
      await loadConfigSummary();
    } catch (error) {
      setFormattedError(setConfigOutput, error);
    } finally {
      setIsRunningConfig(false);
    }
  }, [
    bridgeMissingText,
    configForm.currentProvider,
    invoke,
    loadConfigSummary,
    setFormattedError,
    text.confirmDeleteProvider,
    text.statusDeletingProvider,
    text.validationProviderModelRequired,
  ]);

  const handleImportProviders = useCallback(async () => {
    if (!invoke) {
      setConfigOutput(bridgeMissingText);
      return;
    }

    setIsRunningConfig(true);
    setConfigOutput(text.statusImportingProviders);

    try {
      const response = await invoke<string>('import_opencode_providers_command', {
        form: {
          sourcePath: configForm.importPath,
        },
      });
      setConfigOutput(response);
      await loadConfigSummary();
    } catch (error) {
      setFormattedError(setConfigOutput, error);
    } finally {
      setIsRunningConfig(false);
    }
  }, [
    bridgeMissingText,
    configForm.importPath,
    invoke,
    loadConfigSummary,
    setFormattedError,
    text.statusImportingProviders,
  ]);

  const handleTestCurrentProvider = useCallback(async () => {
    if (!invoke) {
      setConfigOutput(bridgeMissingText);
      return;
    }

    setIsRunningConfig(true);
    setConfigOutput(text.statusTestingProvider);

    try {
      const response = await invoke<string>('test_current_provider_command');
      setConfigOutput(response);
    } catch (error) {
      setFormattedError(setConfigOutput, error);
    } finally {
      setIsRunningConfig(false);
    }
  }, [bridgeMissingText, invoke, setFormattedError, text.statusTestingProvider]);

  const handleCreateAndUseSession = useCallback(async () => {
    if (!invoke) {
      setTimelineOutput(bridgeMissingText);
      return;
    }

    setIsRunningTimeline(true);
    setTimelineOutput(text.statusCreatingSession);

    try {
      const response = await invoke<string>('create_session_command');
      const sessionMatch = response.match(/created session: (session-[^\s]+)/);
      if (!sessionMatch) {
        setTimelineOutput(`${text.validationSessionParseFailed} ${response}`);
        return;
      }

      const sessionId = sessionMatch[1];
      setTimelineForm((previous) => ({ ...previous, sessionId }));

      await refreshSessionOptions(sessionId);
      await refreshTimeline(sessionId);
    } catch (error) {
      setFormattedError(setTimelineOutput, error);
    } finally {
      setIsRunningTimeline(false);
    }
  }, [
    bridgeMissingText,
    invoke,
    refreshSessionOptions,
    refreshTimeline,
    setFormattedError,
    text.statusCreatingSession,
    text.validationSessionParseFailed,
  ]);

  const handleSendSessionMessage = useCallback(async () => {
    if (!invoke) {
      setTimelineOutput(bridgeMissingText);
      return;
    }

    if (!timelineForm.sessionId.trim() || !timelineForm.message.trim()) {
      setTimelineOutput(text.validationSessionAndMessageRequired);
      return;
    }

    const attachmentPaths = timelineForm.attachments
      .split('\n')
      .map((path) => path.trim())
      .filter(Boolean);

    setIsRunningTimeline(true);
    setTimelineOutput(text.statusSendingMessage);
    setGeneralOutput(text.statusPreviewingIntake);

    try {
      const preview = await invoke<string>('preview_session_intake_command', {
        form: {
          sessionId: timelineForm.sessionId.trim(),
          userMessage: timelineForm.message.trim(),
          attachmentPaths,
        },
      });
      setGeneralOutput(preview);

      const response = await invoke<string>('send_session_message_command', {
        form: {
          sessionId: timelineForm.sessionId.trim(),
          userMessage: timelineForm.message.trim(),
          attachmentPaths,
        },
      });

      setTimelineOutput(response);
      setTimelineForm((previous) => ({ ...previous, message: '' }));
      await refreshSessionOptions(timelineForm.sessionId.trim());
      await refreshTimeline(timelineForm.sessionId.trim());
    } catch (error) {
      setFormattedError(setTimelineOutput, error);
      setFormattedError(setGeneralOutput, error);
    } finally {
      setIsRunningTimeline(false);
    }
  }, [
    bridgeMissingText,
    invoke,
    refreshSessionOptions,
    refreshTimeline,
    setFormattedError,
    t,
    text.statusPreviewingIntake,
    text.statusSendingMessage,
    text.validationSessionAndMessageRequired,
    timelineForm.attachments,
    timelineForm.message,
    timelineForm.sessionId,
  ]);

  useEffect(() => {
    if (!invoke) {
      setGeneralOutput(bridgeMissingText);
      setConfigOutput(bridgeMissingText);
      setTimelineOutput(bridgeMissingText);
      return;
    }

    void loadConfigSummary();
    void refreshSessionOptions();
  }, [bridgeMissingText, invoke, loadConfigSummary, refreshSessionOptions]);

  return (
    <div className="w-full max-w-5xl space-y-8">
      <header className="space-y-2">
        <h2 className="font-headline text-4xl font-extrabold tracking-tighter text-on-surface">
          {t('settings.debug.title')}
        </h2>
        <p className="max-w-2xl text-sm leading-relaxed text-on-surface-variant">
          {t('settings.debug.description')}
        </p>
      </header>

      <section className="rounded-sm border border-outline-variant/20 bg-surface-container p-6">
        <div className="mb-4 flex items-center justify-between">
          <h3 className="font-headline text-lg font-bold text-on-surface">
            {text.quickActionsTitle}
          </h3>
          <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-on-surface-variant">
            {text.quickActionsBadge}
          </span>
        </div>

        <p className="mb-4 text-sm text-on-surface-variant">{text.quickActionsDescription}</p>

        <div className="grid gap-3 md:grid-cols-3 lg:grid-cols-4">
          {quickActions.map((action) => (
            <button
              key={action.command}
              className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-4 py-3 text-left text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
              disabled={isRunningGeneral || !invoke}
              onClick={() => {
                void handleRunQuickAction(action);
              }}
              type="button"
            >
              {quickActionLabel(action.command, t)}
            </button>
          ))}
      </div>

        <div className="mt-4 grid gap-3 md:grid-cols-[1fr_auto]">
          <input
            className="rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm text-on-surface outline-none focus:border-primary/40"
            onChange={(event) => setSourceId(event.target.value)}
            placeholder={text.sourceIdPlaceholder}
            value={sourceId}
          />
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-4 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningGeneral || !invoke}
            onClick={() => {
              void handleListChunks();
            }}
            type="button"
          >
            {text.actionListChunks}
          </button>
        </div>
      </section>

      <section className="rounded-sm border border-outline-variant/20 bg-surface-container p-6">
        <h3 className="mb-2 font-headline text-lg font-bold text-on-surface">{text.configTitle}</h3>
        <p className="mb-4 text-sm text-on-surface-variant">{text.configDescription}</p>

        <div className="grid gap-4 md:grid-cols-2">
          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant">
            <span>{text.configCurrentProvider}</span>
            <select
              className="w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) => {
                void handleConfigProviderChange(event.target.value);
              }}
              value={configForm.currentProvider}
            >
              {providerOptions.map((providerId) => (
                <option key={providerId} value={providerId}>
                  {providerId}
                </option>
              ))}
            </select>
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant">
            <span>{text.configCurrentModel}</span>
            <select
              className="w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) => {
                void handleConfigModelChange(event.target.value);
              }}
              value={configForm.currentModel}
            >
              {modelOptions.map((modelId) => (
                <option key={modelId} value={modelId}>
                  {modelId}
                </option>
              ))}
            </select>
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant">
            <span>{text.configProviderName}</span>
            <input
              className="w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) =>
                setConfigForm((previous) => ({
                  ...previous,
                  providerName: event.target.value,
                }))
              }
              value={configForm.providerName}
            />
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant">
            <span>{text.configProviderNpm}</span>
            <input
              className="w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) =>
                setConfigForm((previous) => ({
                  ...previous,
                  providerNpm: event.target.value,
                }))
              }
              value={configForm.providerNpm}
            />
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant md:col-span-2">
            <span>{text.configBaseUrl}</span>
            <input
              className="w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) =>
                setConfigForm((previous) => ({
                  ...previous,
                  baseUrl: event.target.value,
                }))
              }
              value={configForm.baseUrl}
            />
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant md:col-span-2">
            <span>{text.configApiKey}</span>
            <input
              className="w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) =>
                setConfigForm((previous) => ({
                  ...previous,
                  apiKey: event.target.value,
                }))
              }
              type="password"
              value={configForm.apiKey}
            />
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant md:col-span-2">
            <span>{text.configProviderJson}</span>
            <textarea
              className="min-h-28 w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 font-mono text-xs font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) =>
                setConfigForm((previous) => ({
                  ...previous,
                  providerJson: event.target.value,
                }))
              }
              value={configForm.providerJson}
            />
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant md:col-span-2">
            <span>{text.configImportPath}</span>
            <input
              className="w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) =>
                setConfigForm((previous) => ({
                  ...previous,
                  importPath: event.target.value,
                }))
              }
              placeholder={text.configImportPathPlaceholder}
              value={configForm.importPath}
            />
          </label>
        </div>

        <div className="mt-4 grid gap-3 md:grid-cols-3 lg:grid-cols-6">
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningConfig || !invoke}
            onClick={() => {
              void loadConfigSummary();
            }}
            type="button"
          >
            {text.actionLoadConfig}
          </button>
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningConfig || !invoke}
            onClick={() => {
              void handleCreateProvider();
            }}
            type="button"
          >
            {text.actionNewProvider}
          </button>
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningConfig || !invoke}
            onClick={() => {
              void handleSaveCurrentProvider();
            }}
            type="button"
          >
            {text.actionSaveProvider}
          </button>
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningConfig || !invoke}
            onClick={() => {
              void handleDeleteProvider();
            }}
            type="button"
          >
            {text.actionDeleteProvider}
          </button>
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningConfig || !invoke}
            onClick={() => {
              void handleImportProviders();
            }}
            type="button"
          >
            {text.actionImportProviders}
          </button>
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningConfig || !invoke}
            onClick={() => {
              void handleTestCurrentProvider();
            }}
            type="button"
          >
            {text.actionTestProvider}
          </button>
        </div>
      </section>

      <section className="rounded-sm border border-outline-variant/20 bg-surface-container p-6">
        <h3 className="mb-3 font-headline text-lg font-bold text-on-surface">
          {text.configOutputTitle}
        </h3>
        <pre className="min-h-36 overflow-auto rounded-sm bg-surface-container-low p-4 font-mono text-xs text-on-surface-variant">
          {configOutput}
        </pre>
      </section>

      <section className="rounded-sm border border-outline-variant/20 bg-surface-container p-6">
        <h3 className="mb-2 font-headline text-lg font-bold text-on-surface">{text.timelineTitle}</h3>
        <p className="mb-4 text-sm text-on-surface-variant">{text.timelineDescription}</p>

        <div className="grid gap-4 md:grid-cols-2">
          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant md:col-span-2">
            <span>{text.timelineExistingSessions}</span>
            <select
              className="w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) => {
                const sessionId = event.target.value;
                setTimelineForm((previous) => ({ ...previous, sessionId }));
                void refreshTimeline(sessionId);
              }}
              value={timelineForm.sessionId}
            >
              <option value="">{text.timelineSelectSession}</option>
              {sessionOptions.map((session) => (
                <option key={session.sessionId} value={session.sessionId}>
                  {session.label}
                </option>
              ))}
            </select>
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant md:col-span-2">
            <span>{text.timelineSessionId}</span>
            <input
              className="w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) =>
                setTimelineForm((previous) => ({
                  ...previous,
                  sessionId: event.target.value,
                }))
              }
              placeholder={text.timelineSessionIdPlaceholder}
              value={timelineForm.sessionId}
            />
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant md:col-span-2">
            <span>{text.timelineMessage}</span>
            <textarea
              className="min-h-24 w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) =>
                setTimelineForm((previous) => ({
                  ...previous,
                  message: event.target.value,
                }))
              }
              placeholder={text.timelineMessagePlaceholder}
              value={timelineForm.message}
            />
          </label>

          <label className="space-y-1 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant md:col-span-2">
            <span>{text.timelineAttachments}</span>
            <textarea
              className="min-h-24 w-full rounded-sm border border-outline-variant/30 bg-surface-container-low px-3 py-2 text-sm font-normal normal-case tracking-normal text-on-surface outline-none focus:border-primary/40"
              onChange={(event) =>
                setTimelineForm((previous) => ({
                  ...previous,
                  attachments: event.target.value,
                }))
              }
              placeholder={text.timelineAttachmentsPlaceholder}
              value={timelineForm.attachments}
            />
          </label>
        </div>

        <div className="mt-4 grid gap-3 md:grid-cols-4">
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningTimeline || !invoke}
            onClick={() => {
              void handleCreateAndUseSession();
            }}
            type="button"
          >
            {text.actionCreateAndUseSession}
          </button>
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningTimeline || !invoke}
            onClick={() => {
              void refreshSessionOptions(timelineForm.sessionId);
            }}
            type="button"
          >
            {text.actionRefreshSessions}
          </button>
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningTimeline || !invoke}
            onClick={() => {
              void handleSendSessionMessage();
            }}
            type="button"
          >
            {text.actionSendMessage}
          </button>
          <button
            className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-3 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
            disabled={isRunningTimeline || !invoke}
            onClick={() => {
              void refreshTimeline(timelineForm.sessionId);
            }}
            type="button"
          >
            {text.actionRefreshTimeline}
          </button>
        </div>
      </section>

      <section className="rounded-sm border border-outline-variant/20 bg-surface-container p-6">
        <h3 className="mb-3 font-headline text-lg font-bold text-on-surface">
          {text.timelineOutputTitle}
        </h3>
        <pre className="min-h-40 overflow-auto rounded-sm bg-surface-container-low p-4 font-mono text-xs text-on-surface-variant">
          {timelineOutput}
        </pre>
      </section>

      <section className="rounded-sm border border-outline-variant/20 bg-surface-container p-6">
        <h3 className="mb-3 font-headline text-lg font-bold text-on-surface">
          {text.outputGeneralTitle}
        </h3>
        <pre className="min-h-40 overflow-auto rounded-sm bg-surface-container-low p-4 font-mono text-xs text-on-surface-variant">
          {generalOutput}
        </pre>
      </section>
    </div>
  );
}
