import { create } from "zustand";
import { commands, events } from "@/bindings";

interface UpdateStore {
  /** Newer published version, or null when this build is current. */
  availableVersion: string | null;
  checking: boolean;
  initialized: boolean;
  initialize: () => Promise<void>;
  /** Asks the backend to check now. Resolves to the newer version, or null. */
  checkNow: () => Promise<string | null>;
  /** Opens Terminal running the installer; hushtype quits and reopens. */
  runInstaller: () => Promise<void>;
}

export const useUpdateStore = create<UpdateStore>((set, get) => ({
  availableVersion: null,
  checking: false,
  initialized: false,

  initialize: async () => {
    if (get().initialized) return;
    set({ initialized: true });
    try {
      set({ availableVersion: await commands.getAvailableUpdate() });
    } catch (error) {
      console.error("Failed to read the update state:", error);
    }
    await events.updateAvailable.listen((event) => {
      set({ availableVersion: event.payload.version });
    });
  },

  checkNow: async () => {
    set({ checking: true });
    try {
      const result = await commands.checkForUpdate();
      if (result.status === "error") {
        throw new Error(result.error);
      }
      set({ availableVersion: result.data });
      return result.data;
    } finally {
      set({ checking: false });
    }
  },

  runInstaller: async () => {
    const result = await commands.runUpdateInstaller();
    if (result.status === "error") {
      throw new Error(result.error);
    }
  },
}));
