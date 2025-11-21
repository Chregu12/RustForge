import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
    root: 'resources',
    publicDir: '../public',
    build: {
        outDir: '../public/build',
        emptyOutDir: true,
        manifest: true,
        rollupOptions: {
            input: {
                app: path.resolve(__dirname, 'resources/js/app.js'),
            },
        },
    },
    server: {
        port: 5173,
        strictPort: true,
    },
});
