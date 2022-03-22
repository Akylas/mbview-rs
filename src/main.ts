import App from './components/App.svelte';
import { sortBy } from './utils';

Array.prototype.sortBy = function (cfg) {
  return sortBy(this, cfg);
};

import { init, getLocaleFromNavigator, addMessages } from 'svelte-i18n';
import './carbon.css';
// import './carbon.scss';

import en from './i18n/en.json';
import fr from './i18n/fr.json';

addMessages('en', en);
addMessages('fr', fr);

// SUPPORTED_LOCALES.forEach(l=>{
// addMessages(l, require(`./i18n/${l}json`));
/* @vite-ignore */
// register(l, () => import(`./i18n/${l}json`));
// })
// console.log('SUPPORTED_LOCALES', SUPPORTED_LOCALES)

init({
  fallbackLocale: 'en',
  initialLocale: getLocaleFromNavigator(),
});

const app = new App({
  target: document.body,
});

export default app;
