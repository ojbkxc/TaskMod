#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""给 client.rs 的 chat 数据结构批量加 derive"""
import io, re

PATH = r"D:\GitHub\TaskMod\frontend\src\api\client.rs"
with io.open(PATH, "r", encoding="utf-8") as f:
    content = f.read()

# 需要 Default 的结构体
NEED_DEFAULT = {
    "ChatSession", "Preset", "Memory", "Skill", "SkillVariable", "SavedItem",
    "Project", "Scenario", "PromptSettings", "McpServer",
}
# 需要 PartialEq 的结构体
NEED_EQ = {"AiProvider", "ChatSession"}

lines = content.split("\n")
out = []
i = 0
while i < len(lines):
    line = lines[i]
    out.append(line)
    # 找到 derive 行，检查下一个非空行是否是 pub struct X
    if line.strip().startswith("#[derive(") and "Default" not in line and "PartialEq" not in line:
        j = i + 1
        while j < len(lines) and lines[j].strip() == "":
            j += 1
        m = None
        if j < len(lines):
            m = re.match(r"\s*pub struct (\w+)", lines[j])
        if m and m.group(1) in NEED_DEFAULT:
            add = "Default"
            if m.group(1) in NEED_EQ:
                add += ", PartialEq"
            # 替换 derive 行：在最后一个 ) 前插入
            stripped = line.strip()
            if stripped.endswith(")"):
                new_line = line.rstrip()
                new_line = new_line[:-1] + ", " + add + ")"
                out[-1] = new_line
        elif m and m.group(1) in NEED_EQ:
            stripped = line.strip()
            if stripped.endswith(")"):
                new_line = line.rstrip()
                new_line = new_line[:-1] + ", PartialEq)"
                out[-1] = new_line
    i += 1

with io.open(PATH, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(out))

print("done")
