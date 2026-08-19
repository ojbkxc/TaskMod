#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
library.rs 列表组件 for->map 转换（精确版）
模式: for X in &props.LIST { ... } -> {props.LIST.iter().map(|X| { 克隆; rsx! { ... } })}
"""
import io, re

PATH = r"D:\GitHub\TaskMod\frontend\src\pages\library.rs"
with io.open(PATH, "r", encoding="utf-8") as f:
    content = f.read()

def find_block(content, for_line_pat):
    """找到 for 行, 返回 (start, end) 块范围(含 for 行和闭合 })"""
    m = re.search(for_line_pat, content)
    if not m:
        return None
    start = m.start()
    # 花括号平衡从 for 行的 '{' 之后开始
    depth = 0
    started = False
    i = start
    in_str = None
    while i < len(content):
        ch = content[i]
        if in_str:
            if ch == '\\':
                i += 2; continue
            if ch == in_str:
                in_str = None
        else:
            if ch in ('"', "'"):
                in_str = ch
            elif ch == '{':
                depth += 1; started = True
            elif ch == '}':
                depth -= 1
                if started and depth == 0:
                    return start, i + 1
        i += 1
    return None

def convert_list(content, var, list_field):
    """转换单个列表组件"""
    pat = r"(?m)^(\s*)for " + var + r" in &props\." + list_field + r" \{\n"
    m = re.search(pat, content)
    if not m:
        print(f"  [跳过] 未找到 for {var}")
        return content
    indent = m.group(1)
    block_range = find_block(content, pat)
    if not block_range:
        print(f"  [跳过] 无法定位块 {var}")
        return content
    start, end = block_range
    block = content[start:end]
    for_line = block.split("\n", 1)[0]
    inner = block[len(for_line) + 1:]  # 去掉 for 行
    # 去掉块尾闭合 '}' (最后一个非空行)
    inner_lines = inner.rstrip().split("\n")
    # 最后一个 } 是 for 的闭合
    inner = inner.rstrip()
    # 移除末尾的 for 闭合行（内容仅为 {indent}}）
    closing = indent + "}"
    if inner.endswith("\n" + closing):
        inner = inner[:-(len("\n" + closing))]
    inner = inner.rstrip("\n")

    # 替换
    inner = inner.replace("props.on_edit.call(json!({", "on_edit.call(json!({")
    # json! 字段: "f": X.f -> "f": X_clone.f
    inner = re.sub(r'"(\w+)": ' + var + r'\.(\w+)', r'"\1": ' + var + r'_clone.\2', inner)
    # on_delete
    inner = re.sub(r"props\.on_delete\.call\(" + var + r"\.id\.clone\(\)\)", "on_delete.call(" + var + "_id.clone())", inner)

    prefix_lines = [
        indent + "{" + f"props.{list_field}.iter().map(|{var}| {{",
        indent + f"    let {var}_clone = {var}.clone();",
        indent + f"    let {var}_id = {var}.id.clone();",
        indent + "    let on_edit = props.on_edit.clone();",
        indent + "    let on_delete = props.on_delete.clone();",
        indent + "    rsx! {",
    ]
    prefix = "\n".join(prefix_lines) + "\n"
    suffix = "\n" + indent + "    }\n" + indent + "}})}\n"

    new_block = prefix + inner + suffix
    return content[:start] + new_block + content[end:]

# 转换各组件
for var, lf in [("preset", "presets"), ("skill", "skills"), ("project", "projects"),
                ("item", "items")]:
    print(f"转换 {var} / {lf}")
    content = convert_list(content, var, lf)

with io.open(PATH, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("done")
