#!/bin/bash
# TaskMod Frontend Build Script
# 编译 Dioxus 前端并部署到 server/static/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$SCRIPT_DIR"
STATIC_DIR="$SCRIPT_DIR/../server/static"
BACKUP_DIR="$SCRIPT_DIR/../server/static_backup"

echo "=== TaskMod Frontend Build ==="
echo "Frontend dir: $FRONTEND_DIR"
echo "Static dir: $STATIC_DIR"

# 备份现有静态文件
if [ -d "$STATIC_DIR" ] && [ ! -d "$BACKUP_DIR" ]; then
    echo "Backing up existing static files..."
    cp -r "$STATIC_DIR" "$BACKUP_DIR"
fi

# 安装 Dioxus CLI（如果未安装）
if ! command -v dx &> /dev/null; then
    echo "Installing Dioxus CLI..."
    cargo install dioxus-cli
fi

# 编译前端
echo "Building Dioxus frontend..."
cd "$FRONTEND_DIR"
dx build --release

# 复制编译产物到 static 目录
echo "Deploying to static directory..."
mkdir -p "$STATIC_DIR"

# Dioxus 输出目录
DIST_DIR="$FRONTEND_DIR/dist"

if [ -d "$DIST_DIR" ]; then
    # 清理旧文件（保留备份）
    rm -f "$STATIC_DIR/index.html"
    rm -f "$STATIC_DIR/style.css"
    rm -f "$STATIC_DIR/app.js"

    # 复制新文件
    cp -r "$DIST_DIR"/* "$STATIC_DIR/"

    echo "Build complete! Frontend deployed to $STATIC_DIR"
    echo ""
    echo "Files deployed:"
    ls -la "$STATIC_DIR/"
else
    echo "Error: Build output not found at $DIST_DIR"
    exit 1
fi
