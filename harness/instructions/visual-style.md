---
inclusion: fileMatch
fileMatchPattern: ["*.html", "*.md"]
---

# Visual Style Guide

Technical Brutalist-Minimalist aesthetic for artifacts.

## Colors

- Background: #FBFBFB (off-white)
- Text: #111111 (near-black)
- Secondary: #666666 (muted gray)
- Accent: #F0F0F0 (light gray)
- Borders: 1px solid #111111 or #EEEEEE

## Typography

- Font: System monospace only (ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas)
- NO border-radius
- NO box-shadow
- NO gradients

## Layout

- Flat, sharp, asymmetrical
- Whitespace-heavy
- Left-aligned

## Charts (HTML5 Native)

Use native elements only - NO Chart.js:

```html
<progress value="50" max="100"></progress>
<meter value="75" min="0" max="100"></meter>
<div style="width: 50%;"></div>
```

## Slides (Reveal.js)

Inject markdown between `<!-- {{SLIDES_CONTENT}} -->` markers.

## Diagrams (Mermaid)

- Max 3 words per node
- No decorative transitions

## Skill Reference

For execution, use [knowledge-base](../skills/knowledge-base/SKILL.md)