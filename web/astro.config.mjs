import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';

const isGitHubActions = process.env.GITHUB_ACTIONS === 'true';
const repoName = process.env.GITHUB_REPOSITORY?.split('/')[1] ?? 'Elemental';
const repoOwner = process.env.GITHUB_REPOSITORY?.split('/')[0]?.toLowerCase() ?? 'electronicwave';

const site = process.env.SITE_URL ?? `https://${repoOwner}.github.io`;
const base = process.env.BASE_PATH ?? (isGitHubActions ? `/${repoName}` : '/');

export default defineConfig({
  site,
  base,
  output: 'static',
  trailingSlash: 'ignore',
  markdown: {
    shikiConfig: {
      themes: {
        light: 'github-light-default',
        dark: 'github-dark-default',
      },
      wrap: false,
    },
  },
  vite: {
    plugins: [tailwindcss()],
    optimizeDeps: {
      include: ['mermaid'],
    },
    server: {
      fs: {
        allow: ['..'],
      },
    },
  },
});
