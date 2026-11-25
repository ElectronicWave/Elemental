import type { CollectionEntry } from 'astro:content';
import { loadDocsMeta, type DocSectionMeta } from './docs-meta';

export type DocEntry = CollectionEntry<'docs'>;
export type { DocSectionMeta };

export interface DocMeta {
  readonly id: string;
  readonly slug: string;
  readonly sectionId: string;
  readonly title: string;
  readonly description: string;
  readonly group: string;
  readonly createDate: Date;
  readonly updatedDate: Date;
  readonly readingMinutes: number;
  readonly entry: DocEntry;
}

export interface DocGroup {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly items: readonly DocMeta[];
}

export const DOC_SECTIONS: readonly DocSectionMeta[] = loadDocsMeta().sections;

const SECTION_INDEX: ReadonlyMap<string, { readonly meta: DocSectionMeta; readonly order: number }> = new Map(
  DOC_SECTIONS.map((meta, index) => [meta.id, { meta, order: index }]),
);

function getSectionEntry(slug: string): { readonly meta: DocSectionMeta; readonly order: number } {
  const sectionId = slug.split('/')[0] ?? '';
  const found = SECTION_INDEX.get(sectionId);
  if (!found) {
    throw new Error(`Document "${slug}" does not belong to any section declared in docs/meta.yaml`);
  }
  return found;
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
  const { meta } = getSectionEntry(slug);
  return {
    id: entry.id,
    slug,
    sectionId: meta.id,
    title: entry.data.title,
    description: entry.data.description,
    group: meta.label,
    createDate: entry.data['create-date'],
    updatedDate: entry.data['updated-date'],
    readingMinutes: estimateReadingMinutes(entry.body ?? ''),
    entry,
  };
}

export function sortDocs(entries: readonly DocEntry[]): DocMeta[] {
  return entries.map(toDocMeta).sort((a, b) => {
    const sectionA = getSectionEntry(a.slug);
    const sectionB = getSectionEntry(b.slug);
    if (sectionA.order !== sectionB.order) {
      return sectionA.order - sectionB.order;
    }

    const direction = sectionA.meta.sort === 'asc' ? 1 : -1;
    const createDiff = (a.createDate.getTime() - b.createDate.getTime()) * direction;
    if (createDiff !== 0) {
      return createDiff;
    }

    const updatedDiff = b.updatedDate.getTime() - a.updatedDate.getTime();
    if (updatedDiff !== 0) {
      return updatedDiff;
    }

    return a.slug.localeCompare(b.slug);
  });
}

export function groupDocs(docs: readonly DocMeta[]): DocGroup[] {
  const grouped = new Map<string, DocMeta[]>();
  for (const doc of docs) {
    const list = grouped.get(doc.sectionId);
    if (list) {
      list.push(doc);
    } else {
      grouped.set(doc.sectionId, [doc]);
    }
  }

  return DOC_SECTIONS.flatMap((section) => {
    const items = grouped.get(section.id);
    if (!items || items.length === 0) {
      return [];
    }
    return [{ id: section.id, name: section.label, description: section.description, items }];
  });
}
