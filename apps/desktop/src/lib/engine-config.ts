export interface EnginePickerConfig {
  extension: string;
  defaultDir: string | null;
  badgeColor: string;
  title: string;
}

export function getEngineDefaults(gameDir?: string | null): Record<string, EnginePickerConfig> {
  return {
    flash: {
      extension: "sol",
      defaultDir: "%APPDATA%\\Macromedia\\Flash Player\\#SharedObjects",
      badgeColor: "#f44336",
      title: "Select Flash Save Folder",
    },
    "unreal-engine": {
      extension: "sav",
      defaultDir: "%LOCALAPPDATA%",
      badgeColor: "#1565c0",
      title: "Select Unreal Engine Save Folder",
    },
    sugarcube: {
      extension: "save",
      defaultDir: "%USERPROFILE%\\Downloads",
      badgeColor: "#8b5cf6",
      title: "Select SugarCube Save Folder",
    },
    renpy: {
      extension: "save",
      defaultDir: gameDir ? `${gameDir}\\game\\saves` : "%APPDATA%\\RenPy",
      badgeColor: "#ff7eb3",
      title: "Select Ren'Py Save Folder",
    },
    unity: {
      extension: "json",
      defaultDir: "%LOCALAPPDATA%Low",
      badgeColor: "#222c37",
      title: "Select Unity Save Folder",
    },
    "rpg-maker-mv": {
      extension: "rpgsave",
      defaultDir: gameDir ?? null,
      badgeColor: "#4fc3f7",
      title: "Select RPG Maker MV/MZ Game Folder",
    },
    "rpg-maker-vx-ace": {
      extension: "rvdata2",
      defaultDir: gameDir ?? null,
      badgeColor: "#66bb6a",
      title: "Select RPG Maker VX Ace Game Folder",
    },
    "pixel-game-maker-mv": {
      extension: "json",
      defaultDir: gameDir ?? null,
      badgeColor: "#ff7043",
      title: "Select Pixel Game Maker MV Game Folder",
    },
    "wolf-rpg-editor": {
      extension: "sav",
      defaultDir: gameDir ?? null,
      badgeColor: "#ff9800",
      title: "Select Wolf RPG Editor Game Folder",
    },
    sqlite: {
      extension: "db",
      defaultDir: gameDir ?? null,
      badgeColor: "#003b57",
      title: "Select SQLite Save Folder",
    },
  };
}
