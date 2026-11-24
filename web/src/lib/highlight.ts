function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

const SHELL_COMMANDS = new Set(['cargo', 'git', 'cd', 'rustup']);

export function highlightShellLine(line: string): string {
  const trimmed = line.trim();
  if (!trimmed) return '&nbsp;';
  if (trimmed.startsWith('#')) {
    return `<span class="syn-cm">${escapeHtml(line)}</span>`;
  }

  const tokens = line.split(/(\s+)/);
  let seenCommand = false;

  return tokens
    .map((token) => {
      if (!token || /^\s+$/.test(token)) {
        return token;
      }
      const escaped = escapeHtml(token);
      if (token === '$') {
        return `<span class="syn-cm">$</span>`;
      }
      if (!seenCommand && SHELL_COMMANDS.has(token)) {
        seenCommand = true;
        return `<span class="syn-macro">${escaped}</span>`;
      }
      if (token === '--') {
        return `<span class="syn-cm">--</span>`;
      }
      if (token.startsWith('--') || /^-[a-zA-Z]$/.test(token)) {
        return `<span class="syn-ty">${escaped}</span>`;
      }
      if (/^https?:\/\//.test(token) || /^["'].*["']$/.test(token)) {
        return `<span class="syn-str">${escaped}</span>`;
      }
      if (/^\d+(\.\d+)*(-[A-Za-z0-9.]+)?$/.test(token)) {
        return `<span class="syn-num">${escaped}</span>`;
      }
      if (seenCommand && (token === 'run' || token === 'clone' || token === 'add' || token === 'list')) {
        return `<span class="syn-fn">${escaped}</span>`;
      }
      return `<span class="syn-plain">${escaped}</span>`;
    })
    .join('');
}

export function highlightShellBlock(script: string): string {
  return script
    .split('\n')
    .map((line) => highlightShellLine(line))
    .join('\n');
}
