import type { CollectionEntry } from 'astro:content';

export type DocEntry = CollectionEntry<'docs'>;

export interface DocMeta {
  readonly id: string;
  readonly slug: string;
  readonly title: string;
  readonly description: string;
  readonly group: string;
  readonly order: number;
  readonly readingMinutes: number;
  readonly entry: DocEntry;
}

export interface DocGroup {
  readonly name: string;
  readonly items: readonly DocMeta[];
}

const KNOWN_ORDER: Record<string, number> = {
  'launch-flow': 10,
  refactor: 20,
  'migration/0.3': 30,
  'migration/0.2': 40,
};

function stripMarkdown(text: string): string {
  return text
    .replace(/`([^`]+)`/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/[*_~>#]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

function extractTitle(entry: DocEntry): string {
  if (entry.data.title) {
    return entry.data.title;
  }
  const body = entry.body ?? '';
  const match = body.match(/^#\s+(.+)$/m);
  if (match?.[1]) {
    return stripMarkdown(match[1]);
  }
  const lastSegment = entry.id.split('/').pop() ?? entry.id;
  return lastSegment
    .replace(/\.md$/i, '')
    .replace(/[-_]/g, ' ')
    .replace(/\b\w/g, (ch) => ch.toUpperCase());
}

function extractDescription(entry: DocEntry): string {
  if (entry.data.description) {
    return entry.data.description;
  }
  const body = entry.body ?? '';
  const lines = body.split(/\r?\n/);
  const paragraphLines: string[] = [];
  let inCodeFence = false;

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (line.startsWith('```')) {
      inCodeFence = !inCodeFence;
      continue;
    }
    if (inCodeFence) {
      continue;
    }
    if (!line) {
      if (paragraphLines.length > 0) {
        break;
      }
      continue;
    }
    if (line.startsWith('#') || line.startsWith('|') || line.startsWith('- ') || line.startsWith('>')) {
      if (paragraphLines.length > 0) {
        break;
      }
      continue;
    }
    paragraphLines.push(line);
  }

  const joined = stripMarkdown(paragraphLines.join(' '));
  if (joined.length <= 160) {
    return joined;
  }
  return `${joined.slice(0, 157).trimEnd()}...`;
}

function extractGroup(id: string): string {
  const cleanId = id.replace(/\.md$/i, '');
  if (!cleanId.includes('/')) {
    return 'Architecture & Guides';
  }
  const firstSegment = cleanId.split('/')[0] ?? 'Guides';
  if (firstSegment === 'migration') {
    return 'Migration Guides';
  }
  return firstSegment
    .replace(/[-_]/g, ' ')
    .replace(/\b\w/g, (ch) => ch.toUpperCase());
}

function estimateReadingMinutes(body: string): number {
  const words = body.trim().split(/\s+/).filter(Boolean).length;
  return Math.max(1, Math.round(words / 220));
}

export function normalizeDocSlug(id: string): string {
  return id.replace(/\.md$/i, '').replace(/\\/g, '/');
}

export function toDocMeta(entry: DocEntry): DocMeta {
  const slug = normalizeDocSlug(entry.id);
  const order = entry.data.order ?? KNOWN_ORDER[slug] ?? 100;
  return {
    id: entry.id,
    slug,
    title: extractTitle(entry),
    description: extractDescription(entry),
    group: extractGroup(slug),
    order,
    readingMinutes: estimateReadingMinutes(entry.body ?? ''),
    entry,
  };
}

export function sortDocs(entries: readonly DocEntry[]): DocMeta[] {
  return entries.map(toDocMeta).sort((a, b) => {
    if (a.order !== b.order) {
      return a.order - b.order;
    }
    return a.slug.localeCompare(b.slug);
  });
}

export function groupDocs(docs: readonly DocMeta[]): DocGroup[] {
  const map = new Map<string, DocMeta[]>();
  for (const doc of docs) {
    const list = map.get(doc.group);
    if (list) {
      list.push(doc);
    } else {
      map.set(doc.group, [doc]);
    }
  }
  return Array.from(map.entries()).map(([name, items]) => ({ name, items }));
}
