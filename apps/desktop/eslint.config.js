import js from '@eslint/js';
import globals from 'globals';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist', 'src-tauri'] },
  {
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  },
  {
    files: ['src/**/*.{ts,tsx}'],
    ignores: [
      'src/lib/backend/**',
      // Phase 2/3: remaining direct @tauri-apps touchpoints
      'src/main.tsx',
      'src/App.tsx',
      'src/components/OAuthConsentModal.tsx',
      'src/features/settings/UpdateChecker.tsx',
      'src/lib/contribute.ts',
      'src/hooks/useDomainEvents.ts',
      'src/hooks/useWorkspaceEvents.ts',
      'src/hooks/useOAuthClientEvents.ts',
      'src/hooks/useMetaToolEvents.ts',
      'src/lib/api/serverManager.ts',
    ],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['@tauri-apps/*'],
              message:
                'Import Tauri APIs only from @/lib/backend — see docs/planning/unified-backend-facade.md',
            },
          ],
        },
      ],
    },
  }
);
