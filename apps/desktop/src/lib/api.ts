import { invoke } from "@tauri-apps/api/core";

export interface EngineInfo {
  id: string;
  name: string;
  icon: string;
  supports_debug: boolean;
  save_extensions: string[];
  description: string;
  save_dir_hint: string | null;
  pick_mode: string;
}

export interface SaveFile {
  path: string;
  name: string;
  modified: string;
  size: number;
}

export interface SaveData {
  raw: unknown;
  party: Character[] | null;
  inventory: Inventory | null;
  currency: CurrencyInfo | null;
  variables: Variable[] | null;
  switches: Switch[] | null;
  custom_sections: CustomSection[];
}

export interface Character {
  id: number;
  name: string;
  class_name: string | null;
  level: number;
  exp: number;
  stats: Stat[];
  equipment: EquipSlot[];
  skills: NamedId[];
  states: NamedId[];
}

export interface Stat {
  key: string;
  label: string;
  current: number;
  max: number | null;
}

export interface EquipSlot {
  slot_name: string;
  item_id: number | null;
  item_name: string | null;
  data_class: string;
}

export interface Inventory {
  items: InventoryItem[];
  weapons: InventoryItem[];
  armors: InventoryItem[];
}

export interface InventoryItem {
  id: number;
  name: string;
  quantity: number;
  description: string | null;
  category: string | null;
}

export interface CurrencyInfo {
  label: string;
  amount: number;
}

export interface Variable {
  id: number;
  name: string | null;
  value: unknown;
  group: string | null;
}

export interface Switch {
  id: number;
  name: string | null;
  value: boolean;
}

export interface NamedId {
  id: number;
  name: string;
}

export interface CustomSection {
  key: string;
  label: string;
  data: unknown;
}

export interface NameMap {
  actors: Record<number, string>;
  classes: Record<number, string>;
  items: Record<number, string>;
  weapons: Record<number, string>;
  armors: Record<number, string>;
  skills: Record<number, string>;
  variables: Record<number, string>;
  switches: Record<number, string>;
  custom: Record<string, Record<number, string>>;
}

export interface DiffEntry {
  path: string;
  old_value: unknown;
  new_value: unknown;
}

export interface BackupEntry {
  path: string;
  name: string;
  size: number;
  modified: string;
}

export interface PatchInfo {
  engine: string;
  game_dir: string;
  patches: unknown[];
  applied_at: string;
}

export interface SaveDirEntry {
  name: string;
  path: string;
  is_dir: boolean;
  file_count: number;
}

export interface ScanProgressEvent {
  path: string;
  file_count: number;
  folders_done: number;
  folders_total: number;
}

export interface TableMeta {
  name: string;
  columns: { name: string; col_type: string }[];
  row_count: number;
}

export interface TableRow {
  values: unknown[];
}

export interface TableQueryResult {
  columns: string[];
  rows: TableRow[];
  total_rows: number;
}

export interface CellChange {
  table: string;
  rowid: number;
  column: string;
  value: unknown;
}

export const api = {
  listEngines: () => invoke<EngineInfo[]>("list_engines"),
  detectEngine: (gameDir: string) => invoke<EngineInfo | null>("detect_engine", { gameDir }),
  setGame: (engineId: string, gameDir: string) => invoke<void>("set_game", { engineId, gameDir }),
  listSaves: () => invoke<SaveFile[]>("list_saves"),
  loadSave: (savePath: string) => invoke<SaveData>("load_save", { savePath }),
  saveFile: (savePath: string, data: SaveData) => invoke<string>("save_file", { savePath, data }),
  getNames: () => invoke<NameMap>("get_names"),
  getDiff: (savePath: string) => invoke<DiffEntry[]>("get_diff", { savePath }),
  listBackups: (savePath: string) => invoke<BackupEntry[]>("list_backups", { savePath }),
  restoreBackup: (backupPath: string, savePath: string) =>
    invoke<void>("restore_backup", { backupPath, savePath }),
  browseSaveDir: (dir: string | null, defaultDir: string | null, extension: string) =>
    invoke<[string, SaveDirEntry[]]>("browse_save_dir", { dir, defaultDir, extension }),
  deepScanDir: (dir: string, extension: string) =>
    invoke<void>("deep_scan_dir", { dir, extension }),
  applyDebugPatch: () => invoke<PatchInfo>("apply_debug_patch"),
  revertDebugPatch: (patch: PatchInfo) => invoke<void>("revert_debug_patch", { patch }),
  queryTable: (tableName: string, offset: number, limit: number) =>
    invoke<TableQueryResult>("query_table", { tableName, offset, limit }),
  updateRows: (changes: CellChange[]) => invoke<number>("update_rows", { changes }),
  insertRow: (tableName: string) => invoke<number>("insert_row", { tableName }),
  deleteRows: (tableName: string, rowids: number[]) =>
    invoke<number>("delete_rows", { tableName, rowids }),
};
