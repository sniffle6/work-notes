/** What the attachment viewer can show, and the type its content is served as. */
export type MediaType = { kind: "image" | "video"; mimeType: string };

const VIDEO_EXTENSIONS = ["mp4", "m4v", "mov", "webm"];
const RIFF_FORMAT_OFFSET = 8;
const FTYP_OFFSET = 4;

function hasBytesAt(content: Uint8Array, offset: number, signature: string): boolean {
  if (content.length < offset + signature.length) {
    return false;
  }
  return [...signature].every((character, index) => content[offset + index] === character.charCodeAt(0));
}

/**
 * Identifies an image or video by its leading bytes. Returns `null` for
 * content the viewer cannot show.
 */
export function sniffMediaType(content: Uint8Array): MediaType | null {
  if (hasBytesAt(content, 0, "\x89PNG\r\n\x1a\n")) {
    return { kind: "image", mimeType: "image/png" };
  }
  if (hasBytesAt(content, 0, "\xff\xd8\xff")) {
    return { kind: "image", mimeType: "image/jpeg" };
  }
  if (hasBytesAt(content, 0, "GIF8")) {
    return { kind: "image", mimeType: "image/gif" };
  }
  if (hasBytesAt(content, 0, "RIFF") && hasBytesAt(content, RIFF_FORMAT_OFFSET, "WEBP")) {
    return { kind: "image", mimeType: "image/webp" };
  }
  // MP4 and QuickTime share this container; the webview plays QuickTime only when it is typed as MP4.
  if (hasBytesAt(content, FTYP_OFFSET, "ftyp")) {
    return { kind: "video", mimeType: "video/mp4" };
  }
  // The EBML header of WebM and Matroska.
  if (hasBytesAt(content, 0, "\x1a\x45\xdf\xa3")) {
    return { kind: "video", mimeType: "video/webm" };
  }
  return null;
}

/**
 * Tells whether an attachment is a video before it is downloaded, from
 * Jira's type for it or, when Jira gives none, from its file extension.
 */
export function isVideoAttachment(attachment: { mimeType: string | null; filename: string }): boolean {
  if (attachment.mimeType) {
    return attachment.mimeType.toLowerCase().startsWith("video/");
  }
  const extensionStart = attachment.filename.lastIndexOf(".");
  if (extensionStart < 0) {
    return false;
  }
  return VIDEO_EXTENSIONS.includes(attachment.filename.slice(extensionStart + 1).toLowerCase());
}
