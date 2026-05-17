"use client";
import React, { useState } from "react";
import Editor from "@monaco-editor/react";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";

import { NavBar } from "@/components/tubelight-navbar";
import { LanguageSelector, languageOptions, LanguageKey } from "@/components/LanguageSelector";
import { OutputPanel } from "@/components/OutputPanel";

import { Home, User, Briefcase, Play, Loader } from "lucide-react";

const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

const starterCode: Record<LanguageKey, string> = {
  custom: `// Welcome to the Custom Language!\nfn fibonacci(n) {\n  if (n <= 1) {\n    return n;\n  }\n  return fibonacci(n - 1) + fibonacci(n - 2);\n}\n\nlet result = fibonacci(10);\nprintln("Fibonacci(10) is:", result);\n`,
  rust: `fn main() {\n    println!("Hello from Rust!");\n}`,
  python: `print("Hello from Python!")`,
  c: `#include <stdio.h>\n\nint main() {\n    printf("Hello from C!\\n");\n    return 0;\n}`,
  cpp: `#include <iostream>\nusing namespace std;\n\nint main() {\n    cout << "Hello from C++!" << endl;\n    return 0;\n}`,
  javascript: `console.log("Hello from JavaScript!");`,
  go: `package main\n\nimport "fmt"\n\nfunc main() {\n    fmt.Println("Hello from Go!")\n}`,
  java: `public class Main {\n    public static void main(String[] args) {\n        System.out.println("Hello from Java!");\n    }\n}`,
};

export default function CompilerPage() {
  const [code, setCode] = useState<string>(starterCode.custom);
  const [selectedLanguage, setSelectedLanguage] = useState<LanguageKey>("custom");
  const [output, setOutput] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [executionTime, setExecutionTime] = useState<number | null>(null);
  const [stdinInput, setStdinInput] = useState<string>("");

  const handleLanguageChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const langKey = event.target.value as LanguageKey;
    setSelectedLanguage(langKey);
    setCode(starterCode[langKey]);
    setOutput(null);
    setError(null);
    setExecutionTime(null);
  };

  const handleSubmit = async () => {
    setIsLoading(true);
    setOutput(null);
    setError(null);
    setExecutionTime(null);

    try {
      const response = await fetch(`${API_BASE_URL}/compile`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ code, language: selectedLanguage, input: stdinInput }),
      });
      const result = await response.json();
      setOutput(result.result);
      setError(result.error);
      setExecutionTime(result.execution_time_ms);
    } catch {
      setError("Failed to connect to the server.");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="h-screen bg-gray-100 flex flex-col">
      <NavBar items={[
          { name: "Home", url: "/", icon: Home },
          { name: "About", url: "/about", icon: User },
          { name: "Compiler", url: "/compiler", icon: Briefcase },
      ]} />
      
      <main className="flex-grow flex flex-col p-4">
        <PanelGroup direction="vertical" className="flex-grow rounded-lg overflow-hidden bg-white shadow-md border border-gray-200">
          <Panel defaultSize={65} minSize={20}>
            {/* --- Top Panel: Editor --- */}
            <div className="h-full flex flex-col bg-white">
              <div className="flex items-center justify-between p-3 bg-gray-50 border-b border-gray-200">
                <LanguageSelector selectedLanguage={selectedLanguage} onLanguageChange={handleLanguageChange} />
                <button
                  onClick={handleSubmit}
                  disabled={isLoading}
                  className="px-5 py-2 bg-green-600 text-white rounded-md font-semibold flex items-center justify-center disabled:bg-gray-400 disabled:cursor-not-allowed hover:bg-green-700 transition-colors"
                >
                  {isLoading ? (
                    <>
                      <Loader className="animate-spin mr-2 h-5 w-5" />
                      Running...
                    </>
                  ) : (
                    <>
                      <Play className="mr-2 h-5 w-5" />
                      Run
                    </>
                  )}
                </button>
              </div>
              <div className="flex-grow">
                <Editor
                  language={languageOptions[selectedLanguage].editorLanguage}
                  value={code}
                  onChange={(value: any) => setCode(value || "")}
                  theme="vs-light"
                  options={{ 
                    minimap: { enabled: false }, 
                    fontSize: 16, 
                    wordWrap: "on",
                    padding: { top: 20 },
                    scrollBeyondLastLine: false,
                    automaticLayout: true,
                  }}
                />
              </div>
            </div>
          </Panel>
          <PanelResizeHandle className="h-2 bg-gray-200 hover:bg-black transition-colors" />
          <Panel defaultSize={35} minSize={15}>
            {/* --- Bottom Panel: Input + Output --- */}
            <div className="h-full flex flex-col">
              {/* Stdin Input */}
              <div className="flex-shrink-0 border-b border-gray-200">
                <div className="flex items-center px-4 py-2 bg-gray-50">
                  <span className="text-sm font-semibold text-gray-600 mr-3">Input (stdin):</span>
                </div>
                <textarea
                  value={stdinInput}
                  onChange={(e) => setStdinInput(e.target.value)}
                  placeholder="Provide input for your program here (e.g. test cases)..."
                  className="w-full px-4 py-2 text-sm font-mono bg-white text-gray-800 border-none resize-none focus:outline-none focus:ring-0"
                  rows={3}
                />
              </div>
              {/* Output */}
              <div className="flex-grow overflow-auto">
                <OutputPanel 
                    output={output} 
                    error={error} 
                    executionTime={executionTime} 
                    isLoading={isLoading} 
                />
              </div>
            </div>
          </Panel>
        </PanelGroup>
      </main>
    </div>
  );
}