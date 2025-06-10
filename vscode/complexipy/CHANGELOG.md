# Change Log

All notable changes to the "complexipy" extension will be documented in this file.

Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

## [0.0.1] - 2024-03-21

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
  - Function complexity: Low (< 15), High (≥ 15)
  - Line complexity: Low (≤ 5), High (> 5)