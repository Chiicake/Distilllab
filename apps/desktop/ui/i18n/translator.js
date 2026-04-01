export const DEFAULT_LOCALE = "en";

export const SUPPORTED_LOCALES = ["en", "zh-CN"];

export const LOCALE_FILES = {
  en: new URL("./en.json", import.meta.url),
  "zh-CN": new URL("./zh-CN.json", import.meta.url),
};

export function normalizeLocale(locale) {
  return SUPPORTED_LOCALES.includes(locale) ? locale : DEFAULT_LOCALE;
}

export async function fetchLocaleJson(locale) {
  const response = await fetch(LOCALE_FILES[locale]);

  if (!response.ok) {
    throw new Error(`Failed to load locale ${locale}: ${response.status}`);
  }

  return await response.json();
}

export async function loadLocaleDictionaries(
  locales = SUPPORTED_LOCALES,
  fetchJson = fetchLocaleJson,
) {
  const dictionaries = {};

  for (const locale of locales) {
    try {
      dictionaries[locale] = await fetchJson(locale);
    } catch (error) {
      if (locale === DEFAULT_LOCALE) {
        throw new Error(`Failed to load required locale ${locale}: ${error}`);
      }

      console.warn(`Failed to load locale ${locale}. Falling back to English.`, error);
      dictionaries[locale] = {};
    }
  }

  if (!dictionaries[DEFAULT_LOCALE]) {
    throw new Error(`Failed to load required locale ${DEFAULT_LOCALE}`);
  }

  return dictionaries;
}

export function createTranslator(dictionaries, locale) {
  const normalizedLocale = normalizeLocale(locale);
  const english = dictionaries[DEFAULT_LOCALE] ?? {};
  const selected = dictionaries[normalizedLocale] ?? english;

  return function translate(key, replacements = {}) {
    const template = selected[key] ?? english[key] ?? key;
    return formatMessage(template, replacements);
  };
}

function formatMessage(template, replacements) {
  return String(template).replace(/\{(\w+)\}/g, (match, key) => {
    if (Object.prototype.hasOwnProperty.call(replacements, key)) {
      return String(replacements[key]);
    }

    return match;
  });
}
