import { writable } from "svelte/store";

export function createCaptureDraftStore(initialValue = "") {
  const { subscribe, set, update } = writable(initialValue);

  return {
    subscribe,
    set,
    update,
    clear: () => set(""),
  };
}
