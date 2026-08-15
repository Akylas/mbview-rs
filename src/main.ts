import { addMessages, getLocaleFromNavigator, init } from 'svelte-i18n';
import App from './components/App.svelte';
import en from './i18n/en.json';
import fr from './i18n/fr.json';
import './theme.css';
import { sortBy } from './utils';

Array.prototype.sortBy = function (cfg) {
  return sortBy(this, cfg);
};

addMessages('en', en);
addMessages('fr', fr);

init({
  fallbackLocale: 'en',
  initialLocale: getLocaleFromNavigator(),
});

const app = new App({
  target: document.body,
});

export default app;
