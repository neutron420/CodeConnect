"use client";
import React from "react";

// Defines the languages your compiler supports.
export const languageOptions = {
  custom: { name: "Custom", editorLanguage: "javascript" },
  rust: { name: "Rust", editorLanguage: "rust" },
  python: { name: "Python", editorLanguage: "python" },
  c: { name: "C", editorLanguage: "c" },
  cpp: { name: "C++", editorLanguage: "cpp" },
  javascript: { name: "JavaScript", editorLanguage: "javascript" },
  go: { name: "Go", editorLanguage: "go" },
  java: { name: "Java", editorLanguage: "java" },
};
export type LanguageKey = keyof typeof languageOptions;

interface LanguageSelectorProps {
  selectedLanguage: LanguageKey;
  onLanguageChange: (event: React.ChangeEvent<HTMLSelectElement>) => void;
}

export const LanguageSelector: React.FC<LanguageSelectorProps> = ({ selectedLanguage, onLanguageChange }) => {
  return (
    <select
      value={selectedLanguage}
      onChange={onLanguageChange}
      className="px-4 py-2 border border-gray-300 rounded-md bg-white text-black focus:outline-none focus:ring-2 focus:ring-black"
    >
      {Object.entries(languageOptions).map(([key, { name }]) => (
        <option key={key} value={key}>{name}</option>
      ))}
    </select>
  );
};