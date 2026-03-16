#!/bin/bash
# Download fonts from Google Fonts that are commonly used in DOCX documents.
# These are metric-compatible alternatives to Microsoft Office fonts.
#
# Usage: ./scripts/download-fonts.sh [cache_dir]
#   cache_dir defaults to ~/.local/share/s1engine/fonts

set -euo pipefail

CACHE_DIR="${1:-${HOME}/.local/share/s1engine/fonts}"
mkdir -p "$CACHE_DIR"

echo "Font cache directory: $CACHE_DIR"

# Google Fonts API base URL for downloading font files
GOOGLE_FONTS_BASE="https://fonts.google.com/download?family="
GOOGLE_FONTS_API="https://fonts.googleapis.com/css2?family="

download_font() {
    local family="$1"
    local url_family="${family// /+}"
    local target_dir="$CACHE_DIR/$family"

    if [ -d "$target_dir" ] && [ "$(ls -A "$target_dir" 2>/dev/null)" ]; then
        echo "  SKIP: $family (already cached)"
        return 0
    fi

    echo "  Downloading: $family..."
    local zip_file="/tmp/s1_font_${url_family}.zip"

    # Download the font family ZIP from Google Fonts
    if curl -sL -o "$zip_file" "${GOOGLE_FONTS_BASE}${url_family}" 2>/dev/null; then
        mkdir -p "$target_dir"
        # Extract only TTF/OTF files
        if unzip -qo "$zip_file" "*.ttf" "*.otf" -d "$target_dir" 2>/dev/null; then
            # Move files from subdirectories to target_dir root
            find "$target_dir" -mindepth 2 -name "*.ttf" -o -name "*.otf" | while read f; do
                mv "$f" "$target_dir/" 2>/dev/null || true
            done
            # Clean up empty subdirs
            find "$target_dir" -mindepth 1 -type d -empty -delete 2>/dev/null || true
            local count=$(find "$target_dir" -name "*.ttf" -o -name "*.otf" | wc -l)
            echo "  OK: $family ($count font files)"
        else
            echo "  WARN: $family (failed to extract - may not be on Google Fonts)"
            rm -rf "$target_dir"
        fi
        rm -f "$zip_file"
    else
        echo "  WARN: $family (download failed)"
    fi
}

echo ""
echo "Downloading metric-compatible Microsoft Office font alternatives..."
echo ""

# Carlito — metric-compatible with Calibri (default Office font)
download_font "Carlito"

# Caladea — metric-compatible with Cambria
download_font "Caladea"

# Tinos — metric-compatible with Times New Roman
download_font "Tinos"

# Arimo — metric-compatible with Arial
download_font "Arimo"

# Cousine — metric-compatible with Courier New
download_font "Cousine"

echo ""
echo "Downloading common document fonts available on Google Fonts..."
echo ""

# These are commonly used in DOCX files and available on Google Fonts
download_font "Roboto"
download_font "Open Sans"
download_font "Lato"
download_font "Montserrat"
download_font "Noto Sans"
download_font "Noto Serif"
download_font "Source Sans 3"
download_font "EB Garamond"
download_font "Merriweather"
download_font "PT Sans"
download_font "PT Serif"
download_font "Inconsolata"

echo ""
echo "Done. Fonts cached in: $CACHE_DIR"
echo ""
echo "To use these fonts with s1engine, either:"
echo "  1. Set S1_FONTS_DIR=$CACHE_DIR in your environment"
echo "  2. Call font_db.load_fonts_dir(\"$CACHE_DIR\") in your code"
