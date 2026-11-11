import { describe, expect, it } from "vitest";

import { isVideoAttachment, sniffMediaType } from "./media-type";

function bytes(text: string): Uint8Array {
  return Uint8Array.from(text, (character) => character.charCodeAt(0));
}

describe("sniffMediaType", () => {
  it("identifies images by their leading bytes", () => {
    expect(sniffMediaType(bytes("\x89PNG\r\n\x1a\nrest"))).toEqual({ kind: "image", mimeType: "image/png" });
    expect(sniffMediaType(bytes("\xff\xd8\xff\xe0rest"))).toEqual({ kind: "image", mimeType: "image/jpeg" });
    expect(sniffMediaType(bytes("GIF89a rest"))).toEqual({ kind: "image", mimeType: "image/gif" });
    expect(sniffMediaType(bytes("RIFF\x10\x00\x00\x00WEBPrest"))).toEqual({ kind: "image", mimeType: "image/webp" });
  });

  it("identifies MP4, QuickTime, and WebM videos", () => {
    expect(sniffMediaType(bytes("\x00\x00\x00\x18ftypmp42rest"))).toEqual({ kind: "video", mimeType: "video/mp4" });
    // QuickTime shares the MP4 container, and the webview only plays it when typed as MP4.
    expect(sniffMediaType(bytes("\x00\x00\x00\x14ftypqt  rest"))).toEqual({ kind: "video", mimeType: "video/mp4" });
    expect(sniffMediaType(bytes("\x1a\x45\xdf\xa3rest"))).toEqual({ kind: "video", mimeType: "video/webm" });
  });

  it("returns null for anything else", () => {
    expect(sniffMediaType(bytes("plain text"))).toBeNull();
    expect(sniffMediaType(bytes("RIFF\x10\x00\x00\x00WAVErest"))).toBeNull();
    expect(sniffMediaType(new Uint8Array())).toBeNull();
  });
});

describe("isVideoAttachment", () => {
  it("goes by Jira's type when there is one", () => {
    expect(isVideoAttachment({ mimeType: "video/mp4", filename: "clip" })).toBe(true);
    expect(isVideoAttachment({ mimeType: "text/plain", filename: "notes.mp4" })).toBe(false);
  });

  it("falls back to the file extension", () => {
    expect(isVideoAttachment({ mimeType: null, filename: "Repro.MP4" })).toBe(true);
    expect(isVideoAttachment({ mimeType: null, filename: "screen.webm" })).toBe(true);
    expect(isVideoAttachment({ mimeType: null, filename: "phone.mov" })).toBe(true);
    expect(isVideoAttachment({ mimeType: null, filename: "photo.png" })).toBe(false);
    expect(isVideoAttachment({ mimeType: null, filename: "mp4" })).toBe(false);
  });
});
