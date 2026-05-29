/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_POSTHOG_KEY: string;
  readonly VITE_POSTHOG_HOST: string;
  readonly VITE_ADMIN_WEB: boolean;
  readonly VITE_BUILD_GIT_SHA: string;
  readonly VITE_BUILD_GIT_BRANCH: string;
  readonly VITE_BUILD_COMMIT_TIME: string;
  readonly VITE_BUILD_COMMIT_AT: string;
  readonly VITE_BUILD_TIME: string;
  readonly VITE_BUILD_AT: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
