import DOMPurify from 'dompurify';
import { marked } from 'marked';

/** Markdown to HTML, sanitized: raw HTML such as a script tag never survives. */
export const renderMarkdown = (src: string): string => DOMPurify.sanitize(marked.parse(src, { async: false }));
