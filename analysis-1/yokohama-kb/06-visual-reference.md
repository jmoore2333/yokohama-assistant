# Yokohama Visual Reference Guide

> Guide for reference images extracted from the rulebook PDF.
> These images should be kept alongside the markdown documents.

---

## Recommended Image Extractions

### Essential Reference Images

| Image | PDF Page | Purpose | Filename |
|-------|----------|---------|----------|
| Area Board anatomy | Page 1 | Shows area structure, power track, building site placement | `ref-area-board-anatomy.png` |
| Management Board (Port) | Page 1 | Shows card slot levels (Low/Mid/High) | `ref-port-board.png` |
| Management Board (Lab) | Page 2 | Shows industry value modifiers by position | `ref-lab-board.png` |
| Management Board (Church) | Page 2 | Shows Faith values and rewards | `ref-church-board.png` |
| Management Board (Customs) | Page 2 | Shows Imported Goods and rewards | `ref-customs-board.png` |
| Order Card anatomy | Page 2 | Shows country icon, requirements, rewards | `ref-order-card.png` |
| Technology Card anatomy | Page 2 | Shows industry cost, country, effect | `ref-tech-card.png` |
| Achievement Card anatomy | Page 2 | Shows goal, first/subsequent rewards | `ref-achievement-card.png` |
| Building Site Card | Page 2 | Shows shophouse/trading house VP spaces | `ref-building-site.png` |
| 4-Player Setup Example | Page 4 | Shows complete board layout | `ref-setup-4player.png` |
| Placement Example | Page 6 | Shows assistant placement with payments | `ref-placement-example.png` |
| Movement Example | Page 6 | Shows president movement rules | `ref-movement-example.png` |
| Icon Legend | Page 10 | Shows all game icons and meanings | `ref-icons.png` |

### Component Identification Images

| Image | PDF Page | Purpose | Filename |
|-------|----------|---------|----------|
| Trade Good tokens | Page 1 | Copper, Silk, Tea, Seafood appearance | `ref-trade-goods.png` |
| Player components | Page 1 | President, Assistant, Shophouse, Trading House | `ref-player-tokens.png` |
| Foreign Mercenary tokens | Page 1 | Shows country flags | `ref-mercenaries.png` |
| 5-Power tokens | Page 1 | Front/back appearance | `ref-5power-tokens.png` |
| Money tokens | Page 1 | 1-Yen and 3-Yen coins | `ref-money.png` |

### Area Type Reference Images

| Image | PDF Page | Purpose | Filename |
|-------|----------|---------|----------|
| Copper Mine | Page 10 | Production area, power rewards | `ref-area-copper.png` |
| Silk Mill | Page 11 | Production area, power rewards | `ref-area-silk.png` |
| Tea Plantation | Page 11 | Production area, power rewards | `ref-area-tea.png` |
| Fishing Ground | Page 11 | Production area, power rewards | `ref-area-fishing.png` |
| Bank | Page 11 | Commercial area, power rewards | `ref-area-bank.png` |
| Port | Page 11-12 | Commercial area, card selection | `ref-area-port.png` |
| Laboratory | Page 12 | Commercial area, tech acquisition | `ref-area-lab.png` |
| Employment Agency | Page 12-13 | Commercial area, warehouse retrieval | `ref-area-employment.png` |
| Chinatown | Page 13 | Commercial area, exchange rates | `ref-area-chinatown.png` |
| Church | Page 13 | Commercial area, faith calculation | `ref-area-church.png` |
| Customs | Page 13-14 | Commercial area, imported goods | `ref-area-customs.png` |
| Canal | Page 14 | Special area, crossing rules | `ref-area-canal.png` |

---

## Image Usage by System Component

### VLM State Extraction
Primary references for visual identification:
- `ref-area-board-anatomy.png` - Understanding area structure
- `ref-player-tokens.png` - Identifying piece types
- `ref-trade-goods.png` - Resource counting
- `ref-setup-4player.png` - Board layout patterns

### Clarification UI
Images to show users when asking for verification:
- `ref-icons.png` - "What does this icon mean?"
- `ref-area-*.png` - "Is this the correct area type?"
- `ref-placement-example.png` - "Did placement follow this pattern?"

### Game Engine Validation
Reference for rule implementation:
- `ref-port-board.png` - Card slot positions
- `ref-lab-board.png` - Industry value modifiers
- `ref-church-board.png` - Track positions and rewards
- `ref-customs-board.png` - Track positions and rewards

### Web UI Visualization
Assets for board state display:
- All `ref-area-*.png` - Area type icons
- `ref-player-tokens.png` - Piece representations
- `ref-trade-goods.png` - Resource icons

---

## Extraction Instructions

### Using PDF Tools

```bash
# Extract specific pages as images
pdftoppm -png -r 300 -f 1 -l 1 rulebook.pdf page1
pdftoppm -png -r 300 -f 4 -l 4 rulebook.pdf page4

# Or use Python with pdf2image
from pdf2image import convert_from_path
pages = convert_from_path('rulebook.pdf', 300)
pages[0].save('page1.png', 'PNG')
```

### Manual Cropping

After extracting full pages, crop specific regions:
- Use consistent dimensions for similar content types
- Maintain aspect ratios
- Include small margin around target content
- Save as PNG for quality

### File Organization

```
yokohama-kb/
├── 01-rules-reference.md
├── 02-quick-reference.md
├── 03-state-schema.md
├── 04-strategy-guide.md
├── 05-vlm-prompts.md
├── 06-visual-reference.md (this file)
└── images/
    ├── anatomy/
    │   ├── ref-area-board-anatomy.png
    │   ├── ref-order-card.png
    │   ├── ref-tech-card.png
    │   └── ...
    ├── components/
    │   ├── ref-player-tokens.png
    │   ├── ref-trade-goods.png
    │   └── ...
    ├── areas/
    │   ├── ref-area-copper.png
    │   ├── ref-area-silk.png
    │   └── ...
    └── examples/
        ├── ref-setup-4player.png
        ├── ref-placement-example.png
        └── ref-movement-example.png
```

---

## Image Specifications

### Recommended Settings

| Property | Value | Reason |
|----------|-------|--------|
| Resolution | 300 DPI | Clear detail for VLM |
| Format | PNG | Lossless quality |
| Color Space | sRGB | Standard web display |
| Max Dimension | 2048px | VLM input limits |

### Naming Convention

```
ref-{category}-{specific}.png

Categories:
- area: Area board tiles
- board: Management boards
- card: Card types
- token: Token types
- example: Gameplay examples
- icon: Symbol references
```

---

## Alternative: Keep Full PDF

If image extraction is impractical, a contributor's own copy of the OKAZU Brand English rulebook can serve as reference:
- VLM can process PDF pages directly (Gemini, Claude)
- Reference specific pages in prompts
- Trade-off: Slower processing, more tokens

**Recommendation:** Extract key images for frequently-referenced content, keep PDF for edge cases.

---

## Quick Visual Reference (Text Descriptions)

For cases where images aren't available, these text descriptions can help:

### Area Board Visual Elements
- Top section: Area name (Japanese + English) and illustration
- Middle section: Common area effects (icons)
- Bottom section: Power track (1-5 circles) with reward icons
- Overlay: Building Site card with VP values

### Player Token Shapes
- President: Tall cylinder with domed/pointed top
- Assistant: Short cylinder
- Shophouse: Small house shape
- Trading House: Larger building shape

### Resource Token Colors
- Copper: Orange/brown, octagonal
- Raw Silk: Beige/cream, rectangular
- Tea: Green, leaf-shaped
- Seafood: Blue, fish-shaped
- Imported Goods: Golden/yellow, decorative
- Money: Metallic, coin-shaped (1 or 3)

### Country Icons
- English: Union Jack pattern
- American: Stars and stripes pattern
- French: Tricolor pattern
- German: Black/red/gold pattern
- Dutch: Orange/blue pattern
