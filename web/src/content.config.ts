import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { z } from 'astro/zod';
import { loadDocsMeta } from './lib/docs-meta';

const docsMeta = loadDocsMeta();

const docs = defineCollection({
  loader: glob({
    pattern: docsMeta.sections.map((section) => `${section.id}/**/*.md`),
    base: '../docs',
    generateId: ({ entry }) => entry.replace(/\.md$/i, '').replace(/\\/g, '/'),
  }),
  schema: z.object({
    title: z.string().min(1),
    description: z.string().min(1),
    'create-date': z.coerce.date(),
    'updated-date': z.coerce.date(),
  }),
});

export const collections = { docs };
