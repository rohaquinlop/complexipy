# Change Log

## [0.1.0] - 2025-07-09

### Enhanced
- **Comprehensive hover messages**: Significantly improved hover information for both functions and lines
- **Intelligent complexity analysis**: Enhanced function complexity hover with detailed breakdown including:
  - Control flow structures count
  - Boolean operators count
  - Deeply nested lines detection
  - High-complexity lines identification
- **Smart refactoring recommendations**: Context-aware suggestions based on complexity patterns:
  - Accumulative complexity detection with targeted strategies
  - High complexity urgent refactoring priorities
  - Function decomposition and extraction guidance
- **Detailed line-level insights**: Enhanced line complexity hover with:
  - Complexity source identification (branching, loops, exception handling, etc.)
  - Complexity breakdown and calculation explanation
  - Priority fixes and specific suggestions per complexity type
  - Parent function context information
- **Educational content**: Added comprehensive explanations about cognitive complexity:
  - Formula breakdown (Base + Nesting + Boolean ops + Structural elements)
  - Key factors and penalties explanation
  - Focus strategies for optimization

### Improved
- **Visual clarity**: Better formatting and structure of hover messages
- **Actionable guidance**: More specific and prioritized refactoring recommendations
- **Context awareness**: Hover messages now adapt based on complexity patterns and nesting levels

## [0.0.3] - 2025-07-05

- Threshold adjustment

## [0.0.2] - 2025-06-12

### Added
- Updated branding assets for the extension

## [0.0.1] - 2025-06-09

### Added
- Initial release of complexipy VSCode extension
- Real-time cognitive complexity analysis for Python code
- Visual complexity indicators:
  - Function complexity shown with `ƒ` symbol
  - Line-level complexity shown with `+` symbol
  - Color-coded indicators (green for low complexity, red for high complexity)
- Automatic updates on:
  - File save
  - Active editor change
  - Text changes
- Manual analysis trigger via command palette
- Support for Python files
- Complexity thresholds:
  - Function complexity: Low (≤ 15), High (> 15)
  - Line complexity: Low (≤ 5), High (> 5)