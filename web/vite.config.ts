import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    fs: {
      // Allow serving the generated wasm package outside web/
      allow: ['..'],
    },
  },
  optimizeDeps: {
    exclude: ['@dab/dab-wasm'],
  },
});
