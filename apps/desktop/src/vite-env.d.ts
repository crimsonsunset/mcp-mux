/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_POSTHOG_KEY: string;
  readonly VITE_POSTHOG_HOST: string;
  readonly VITE_ADMIN_WEB: boolean;
  readonly VITE_BUILD_GIT_SHA: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
