import assert from "node:assert/strict";

import {
  createTranslator,
  loadLocaleDictionaries,
} from "./translator.js";

async function testLocaleLoadingFallsBackToEnglish() {
  const dictionaries = await loadLocaleDictionaries(
    ["en", "zh-CN"],
    async (locale) => {
      if (locale === "en") {
        return {
          greeting: "Hello",
          onlyEnglish: "English only",
        };
      }

      throw new Error("zh-CN unavailable");
    },
  );

  assert.equal(dictionaries.en.greeting, "Hello");
  assert.deepEqual(dictionaries["zh-CN"], {});

  const zhTranslator = createTranslator(dictionaries, "zh-CN");
  assert.equal(zhTranslator("greeting"), "Hello");
  assert.equal(zhTranslator("onlyEnglish"), "English only");
}

async function testEnglishIsRequired() {
  await assert.rejects(
    () => loadLocaleDictionaries(["en", "zh-CN"], async () => {
      throw new Error("everything failed");
    }),
    /Failed to load required locale en/,
  );
}

await testLocaleLoadingFallsBackToEnglish();
await testEnglishIsRequired();

console.log("translator tests passed");
