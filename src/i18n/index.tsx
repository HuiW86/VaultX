import { createContext, useContext, useMemo, type ReactNode } from "react";
import en from "./en";
import zhCN from "./zh-CN";
import type { Locale } from "./types";

export type TranslationKey = keyof typeof en;
export type Translations = Record<TranslationKey, string>;

const translations: Record<Locale, Translations> = {
  en,
  "zh-CN": zhCN,
};

type I18nContextValue = {
  locale: Locale;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
};

const I18nContext = createContext<I18nContextValue>({
  locale: "en",
  t: (key) => en[key],
});

export function I18nProvider({
  locale,
  children,
}: {
  locale: Locale;
  children: ReactNode;
}) {
  const value = useMemo<I18nContextValue>(() => {
    const dict = translations[locale] ?? en;
    return {
      locale,
      t: (key, params) => {
        let text = dict[key] ?? en[key] ?? key;
        if (params) {
          for (const [k, v] of Object.entries(params)) {
            text = text.replace(`{${k}}`, String(v));
          }
        }
        return text;
      },
    };
  }, [locale]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useTranslation() {
  return useContext(I18nContext);
}

export type { Locale };
