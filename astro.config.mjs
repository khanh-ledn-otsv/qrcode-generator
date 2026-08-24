import astroReact from "@astrojs/react";
import viteReact from "@vitejs/plugin-react";
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

function reactWithRustCompiler() {
  const integration = astroReact();
  const setup = integration.hooks["astro:config:setup"];

  integration.hooks["astro:config:setup"] = async (options) => {
    await setup({
      ...options,
      updateConfig(config) {
        const plugins = config.vite?.plugins;

        if (plugins) {
          plugins[0] = viteReact({
            compiler: true,
            exclude: /\.astro$/u,
          });
        }

        return options.updateConfig(config);
      },
    });
  };

  return integration;
}

export default defineConfig({
  base: process.env.PUBLIC_BASE_PATH ?? "/",
  integrations: [reactWithRustCompiler()],
  vite: {
    plugins: [tailwindcss()],
  },
  server: {
    host: "127.0.0.1",
    port: 3000,
  },
});
