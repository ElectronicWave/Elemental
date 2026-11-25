import { parse } from 'yaml';
import rawDocsMeta from '../../../docs/meta.yaml?raw';

export type SortDirection = 'asc' | 'desc';

export interface DocSectionMeta {
  readonly id: string;
  readonly label: string;
  readonly description: string;
  readonly sort: SortDirection;
}

export interface DocsMetaConfig {
  readonly sections: readonly DocSectionMeta[];
}

function parseSection(raw: unknown, index: number): DocSectionMeta {
  if (typeof raw !== 'object' || raw === null) {
    throw new Error(`docs/meta.yaml: sections[${index}] must be a mapping`);
  }
  const record = raw as Record<string, unknown>;
  const id = record.id;
  const label = record.label;
  const description = record.description;
  const sort = record.sort;

  if (typeof id !== 'string' || id.trim().length === 0) {
    throw new Error(`docs/meta.yaml: sections[${index}].id must be a non-empty string`);
  }
  if (typeof label !== 'string' || label.trim().length === 0) {
    throw new Error(`docs/meta.yaml: sections[${index}].label must be a non-empty string`);
  }
  if (typeof description !== 'string' || description.trim().length === 0) {
    throw new Error(`docs/meta.yaml: sections[${index}].description must be a non-empty string`);
  }
  if (sort !== 'asc' && sort !== 'desc') {
    throw new Error(`docs/meta.yaml: sections[${index}].sort must be "asc" or "desc"`);
  }

  return { id, label, description, sort };
}

export function loadDocsMeta(): DocsMetaConfig {
  const parsed: unknown = parse(rawDocsMeta);

  if (typeof parsed !== 'object' || parsed === null) {
    throw new Error('docs/meta.yaml: root value must be a mapping');
  }
  const sectionsRaw = (parsed as Record<string, unknown>).sections;
  if (!Array.isArray(sectionsRaw) || sectionsRaw.length === 0) {
    throw new Error('docs/meta.yaml: "sections" must be a non-empty list');
  }

  return {
    sections: sectionsRaw.map((entry, index) => parseSection(entry, index)),
  };
}
