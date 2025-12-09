import os


def count_lines(file_path):
    """统计单个文件的行数信息：总行数、有效代码行数、注释行数"""
    total_lines = 0
    effective_lines = 0
    comment_lines = 0
    in_multiline_comment = False

    with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
        for line in f:
            total_lines += 1
            stripped_line = line.strip()

            # Python 文件处理
            if file_path.endswith(".py"):
                # 空行不统计为注释
                if not stripped_line:
                    continue

                # 单行注释
                if stripped_line.startswith("#"):
                    comment_lines += 1
                else:
                    effective_lines += 1

            # Rust 文件处理
            elif file_path.endswith(".rs"):
                # 处理多行注释
                if "/*" in line:
                    in_multiline_comment = True
                    comment_lines += 1
                    # 如果注释在同一行结束
                    if "*/" in line:
                        in_multiline_comment = False
                        # 检查是否有代码在同一行
                        code_part = line.split("/*")[0].strip() + line.split("*/")[1].strip()
                        if code_part:
                            effective_lines += 1
                    continue

                # 处理多行注释结束
                if "*/" in line:
                    in_multiline_comment = False
                    comment_lines += 1
                    # 检查是否有代码在同一行
                    code_part = line.split("*/")[1].strip()
                    if code_part:
                        effective_lines += 1
                    continue

                # 在多行注释内部
                if in_multiline_comment:
                    comment_lines += 1
                    continue

                # 空行不统计为注释
                if not stripped_line:
                    continue

                # 单行注释
                if (
                    stripped_line.startswith("//")
                    or stripped_line.startswith("///")
                    or stripped_line.startswith("//!")
                ):
                    comment_lines += 1
                else:
                    effective_lines += 1

    return total_lines, effective_lines, comment_lines


def main():
    # 定义要统计的目录和文件类型
    rust_dir = r"d:\SeeSea-1\src"
    python_dir = r"d:\SeeSea-1\seesea\seesea"

    # 初始化统计变量
    total_total = 0
    total_effective = 0
    total_comment = 0

    rust_total = 0
    rust_effective = 0
    rust_comment = 0

    python_total = 0
    python_effective = 0
    python_comment = 0

    # 统计 Rust 文件
    print("统计 Rust 文件...")
    for root, dirs, files in os.walk(rust_dir):
        for file in files:
            if file.endswith(".rs"):
                file_path = os.path.join(root, file)
                total, effective, comment = count_lines(file_path)
                rust_total += total
                rust_effective += effective
                rust_comment += comment

    # 统计 Python 文件
    print("统计 Python 文件...")
    for root, dirs, files in os.walk(python_dir):
        for file in files:
            if file.endswith(".py"):
                file_path = os.path.join(root, file)
                total, effective, comment = count_lines(file_path)
                python_total += total
                python_effective += effective
                python_comment += comment

    # 计算总和
    total_total = rust_total + python_total
    total_effective = rust_effective + python_effective
    total_comment = rust_comment + python_comment

    # 输出结果
    print("\n统计结果（包含注释）：")
    print("Python 文件：")
    print(f"  总行数：{python_total}")
    print(f"  有效代码行数：{python_effective}")
    print(f"  注释行数：{python_comment}")
    print()
    print("Rust 文件：")
    print(f"  总行数：{rust_total}")
    print(f"  有效代码行数：{rust_effective}")
    print(f"  注释行数：{rust_comment}")
    print()
    print("总计：")
    print(f"  总行数：{total_total}")
    print(f"  有效代码行数：{total_effective}")
    print(f"  注释行数：{total_comment}")


if __name__ == "__main__":
    main()
