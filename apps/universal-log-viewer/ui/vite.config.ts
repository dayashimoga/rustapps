import { defineConfig } from 'vite';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [
    svelte({
      preprocess: [vitePreprocess()],
    }),
  ],
  server: {
    port: 1420,
    strictPort: true,
  },
  clearScreen: false,
});
