import { defineConfig } from 'vite';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [
    svelte({
      preprocess: [vitePreprocess()],
    }),
  ],
  server: {
    port: 1423,
    strictPort: true,
  },
  clearScreen: false,
});

