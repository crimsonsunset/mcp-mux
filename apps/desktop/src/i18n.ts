import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import './i18n.types';
import clients from './locales/en/clients.json';
import common from './locales/en/common.json';
import dashboard from './locales/en/dashboard.json';
import featuresets from './locales/en/featuresets.json';
import nav from './locales/en/nav.json';
import registry from './locales/en/registry.json';
import servers from './locales/en/servers.json';
import settings from './locales/en/settings.json';
import spaces from './locales/en/spaces.json';
import workspaces from './locales/en/workspaces.json';

i18n.use(initReactI18next).init({
  lng: 'en',
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
  resources: {
    en: {
      nav,
      common,
      dashboard,
      servers,
      workspaces,
      featuresets,
      clients,
      settings,
      spaces,
      registry,
    },
  },
});

export default i18n;
