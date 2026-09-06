// SPDX-License-Identifier: Apache-2.0

export interface McpClientSnippet {
  id: 'claudeDesktop' | 'claudeCode' | 'cursor';
  labelKey: string;
  content: string;
}

export function buildMcpSnippets(binaryPath: string): McpClientSnippet[] {
  const json = JSON.stringify({ mcpServers: { qoredb: { command: binaryPath } } }, null, 2);
  const shellPath = /\s/.test(binaryPath) ? `"${binaryPath}"` : binaryPath;
  return [
    { id: 'claudeDesktop', labelKey: 'settings.agents.config.claudeDesktop', content: json },
    {
      id: 'claudeCode',
      labelKey: 'settings.agents.config.claudeCode',
      content: `claude mcp add qoredb -- ${shellPath}`,
    },
    { id: 'cursor', labelKey: 'settings.agents.config.cursor', content: json },
  ];
}
