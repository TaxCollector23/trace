/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_TRACE_FIXTURES?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
