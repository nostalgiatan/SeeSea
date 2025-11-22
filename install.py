#!/usr/bin/env python3
"""
SeeSea 安装脚本
自动检查和安装所需依赖，构建并安装 SeeSea 项目
"""

import os
import sys
import subprocess
import importlib
from typing import List, Tuple
import shutil
import hashlib
import time


class Colors:
    """终端颜色输出"""
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BLUE = '\033[94m'
    BOLD = '\033[1m'
    END = '\033[0m'


def print_info(message: str):
    """打印信息消息"""
    print(f"{Colors.BLUE}[INFO]{Colors.END} {message}")


def print_success(message: str):
    """打印成功消息"""
    print(f"{Colors.GREEN}[SUCCESS]{Colors.END} {message}")


def print_warning(message: str):
    """打印警告消息"""
    print(f"{Colors.YELLOW}[WARNING]{Colors.END} {message}")


def print_error(message: str):
    """打印错误消息"""
    print(f"{Colors.RED}[ERROR]{Colors.END} {message}")


def run_command(command: List[str], check: bool = True, capture_output: bool = False) -> subprocess.CompletedProcess:
    """运行命令"""
    try:
        if capture_output:
            result = subprocess.run(command, check=check, capture_output=True, text=True)
        else:
            result = subprocess.run(command, check=check)
        return result
    except subprocess.CalledProcessError as e:
        if check:
            print_error(f"命令执行失败: {' '.join(command)}")
            print_error(f"错误信息: {e}")
            sys.exit(1)
        return e


def is_package_installed(package_name: str) -> bool:
    """检查包是否已安装"""
    try:
        importlib.import_module(package_name)
        return True
    except ImportError:
        # 对于特殊的包名，使用不同的检查方式
        try:
            result = run_command([sys.executable, "-m", "pip", "show", package_name],
                              check=False, capture_output=True)
            return result.returncode == 0
        except:
            return False


def get_file_hash(file_path: str) -> str:
    """计算文件的哈希值"""
    if not os.path.exists(file_path):
        return ""
    hash_md5 = hashlib.md5()
    with open(file_path, "rb") as f:
        for chunk in iter(lambda: f.read(4096), b""):
            hash_md5.update(chunk)
    return hash_md5.hexdigest()


def get_build_state_file() -> str:
    """获取构建状态文件路径"""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(script_dir, ".build_state")


def load_build_state() -> dict:
    """加载构建状态"""
    state_file = get_build_state_file()
    if os.path.exists(state_file):
        try:
            with open(state_file, 'r') as f:
                content = f.read().strip()
                if content:
                    # 使用安全的 JSON 解析替代 eval
                    import json
                    return json.loads(content)
        except (json.JSONDecodeError, Exception):
            pass
    return {}


def save_build_state(state: dict):
    """保存构建状态"""
    state_file = get_build_state_file()
    import json
    with open(state_file, 'w') as f:
        json.dump(state, f)


def need_rebuild() -> bool:
    """检查是否需要重新构建"""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    cargo_file = os.path.join(script_dir, "Cargo.toml")
    pyproject_file = os.path.join(script_dir, "seesea", "pyproject.toml")

    # 检查源文件是否存在
    if not os.path.exists(cargo_file) or not os.path.exists(pyproject_file):
        return True

    state = load_build_state()
    current_hashes = {
        'cargo': get_file_hash(cargo_file),
        'pyproject': get_file_hash(pyproject_file)
    }

    # 如果哈希值变化，需要重新构建
    if state.get('hashes') != current_hashes:
        return True

    # 检查是否有wheel文件
    target_dir = os.path.join(script_dir, "target", "wheels")
    if os.path.exists(target_dir):
        whl_files = [f for f in os.listdir(target_dir) if f.endswith('.whl')]
        if whl_files:
            # 检查wheel文件是否比源文件新
            latest_whl = sorted(whl_files)[-1]
            whl_path = os.path.join(target_dir, latest_whl)
            whl_mtime = os.path.getmtime(whl_path)
            cargo_mtime = os.path.getmtime(cargo_file)
            pyproject_mtime = os.path.getmtime(pyproject_file)

            if whl_mtime > max(cargo_mtime, pyproject_mtime):
                return False

    return True


def check_venv_environment() -> bool:
    """检查是否在虚拟环境中"""
    # 检查标准的虚拟环境指示器
    if hasattr(sys, 'real_prefix') or (hasattr(sys, 'base_prefix') and sys.base_prefix != sys.prefix):
        return True

    # 检查 VIRTUAL_ENV 环境变量
    if os.environ.get('VIRTUAL_ENV'):
        return True

    # 检查 conda 环境
    if os.environ.get('CONDA_DEFAULT_ENV') and os.environ.get('CONDA_PREFIX'):
        return True

    return False


def ask_user_question(question: str) -> bool:
    """询问用户是/否问题"""
    try:
        while True:
            response = input(f"{Colors.YELLOW}{question} (y/n): {Colors.END}").strip().lower()
            if response in ['y', 'yes', '是']:
                return True
            elif response in ['n', 'no', '否']:
                return False
            else:
                print_warning("请输入 'y' 或 'n'")
    except (EOFError, KeyboardInterrupt):
        # 在非交互环境或用户中断时返回默认值
        print_warning("检测到非交互环境或用户中断，选择默认选项")
        return False


def install_package(package_name: str, import_name: str = None) -> bool:
    """安装包"""
    if import_name is None:
        import_name = package_name

    if is_package_installed(import_name):
        print_success(f"{package_name} 已安装")
        return True

    print_info(f"正在安装 {package_name}...")
    try:
        run_command([sys.executable, "-m", "pip", "install", package_name])
        print_success(f"{package_name} 安装成功")
        return True
    except Exception as e:
        print_error(f"{package_name} 安装失败: {e}")
        return False


def check_and_install_dependencies():
    """检查并安装所需依赖"""
    print_info("检查所需依赖...")

    required_packages = [
        ("playwright-python", "playwright"),
        ("maturin[patchelf]", "maturin"),
        ("rich", "rich"),
        ("click", "click"),
    ]

    missing_packages = []

    # 检查每个包
    for package_name, import_name in required_packages:
        if not is_package_installed(import_name):
            missing_packages.append((package_name, import_name))
        else:
            print_success(f"{package_name} 已安装")

    # 如果有缺失的包，询问用户是否安装
    if missing_packages:
        print_warning(f"发现缺失的依赖: {[pkg[0] for pkg in missing_packages]}")
        if ask_user_question("是否安装缺失的依赖？"):
            for package_name, import_name in missing_packages:
                if not install_package(package_name, import_name):
                    print_error(f"安装 {package_name} 失败，安装中止")
                    sys.exit(1)
        else:
            print_error("缺少必要依赖，无法继续安装")
            sys.exit(1)


def setup_playwright():
    """安装和配置 Playwright"""
    print_info("配置 Playwright...")

    if not is_package_installed("playwright"):
        print_error("Playwright 未安装")
        return False

    try:
        # 简化的检查方式：直接尝试安装，让 playwright 自己处理重复安装
        print_info("安装 Playwright 依赖...")
        run_command([sys.executable, "-m", "playwright", "install-deps"], check=False)

        print_info("安装 Playwright Chromium...")
        run_command([sys.executable, "-m", "playwright", "install", "chromium"], check=False)

        print_success("Playwright 配置完成")
        return True
    except Exception as e:
        print_error(f"Playwright 配置失败: {e}")
        return False


def build_with_maturin():
    """使用 Maturin 构建项目"""
    if not is_package_installed("maturin"):
        print_error("Maturin 未安装")
        return False

    # 检查是否需要重新构建
    if not need_rebuild():
        print_success("项目已构建且为最新，跳过构建")
        return True

    print_info("使用 Maturin 构建项目...")

    try:
        # 保存当前目录并在构建后恢复
        script_dir = os.path.dirname(os.path.abspath(__file__))
        original_dir = os.getcwd()
        os.chdir(script_dir)

        try:
            print_info("构建项目...")
            run_command(["maturin", "build", "--release", "--strip"])

            # 保存构建状态
            state = load_build_state()
            cargo_file = os.path.join(script_dir, "Cargo.toml")
            pyproject_file = os.path.join(script_dir, "seesea", "pyproject.toml")
            state['hashes'] = {
                'cargo': get_file_hash(cargo_file),
                'pyproject': get_file_hash(pyproject_file)
            }
            save_build_state(state)

            print_success("Maturin 构建完成")
            return True
        finally:
            # 恢复原始目录
            os.chdir(original_dir)
    except Exception as e:
        print_error(f"Maturin 构建失败: {e}")
        return False


def is_package_installed_with_version(package_name: str, version: str = None) -> bool:
    """检查包是否已安装（可选版本检查）"""
    try:
        result = run_command([sys.executable, "-m", "pip", "show", package_name],
                          check=False, capture_output=True)
        if result.returncode != 0:
            return False

        if version:
            # 检查版本
            lines = result.stdout.split('\n')
            for line in lines:
                if line.startswith('Version:'):
                    installed_version = line.split(':')[1].strip()
                    return installed_version == version
        return True
    except:
        return False


def install_whl_file():
    """安装构建生成的 .whl 文件"""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    target_dir = os.path.join(script_dir, "target", "wheels")

    if not os.path.exists(target_dir):
        print_error(f"构建目标目录不存在: {target_dir}")
        return False

    # 查找 .whl 文件
    whl_files = [f for f in os.listdir(target_dir) if f.endswith('.whl')]

    if not whl_files:
        print_error("未找到 .whl 文件")
        return False

    # 选择最新的 .whl 文件
    whl_file = sorted(whl_files)[-1]
    whl_path = os.path.join(target_dir, whl_file)

    # 从文件名提取包名和版本
    try:
        base_name = whl_file.replace('.whl', '')
        parts = base_name.split('-')
        if len(parts) >= 2:
            package_name = parts[0]
            version = parts[1]

            # 检查是否已安装相同版本
            if is_package_installed_with_version(package_name, version):
                print_success(f"{package_name} 版本 {version} 已安装，跳过")
                return True
    except:
        pass

    try:
        print_info(f"安装 {whl_file}...")
        run_command([sys.executable, "-m", "pip", "install", whl_path, "--force-reinstall"])
        print_success(f"{whl_file} 安装成功")
        return True
    except Exception as e:
        print_error(f"安装 .whl 文件失败: {e}")
        return False


def install_seesea_package():
    """安装 seesea 包"""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    seesea_dir = os.path.join(script_dir, "seesea")

    if not os.path.exists(seesea_dir):
        print_error(f"seesea 目录不存在: {seesea_dir}")
        return False

    # 检查是否已安装 seesea
    if is_package_installed("seesea"):
        # 简化检查：直接尝试重新安装，让 pip 处理重复安装
        print_info("seesea 已安装，更新为最新版本...")
        try:
            original_dir = os.getcwd()
            os.chdir(seesea_dir)
            run_command([sys.executable, "-m", "pip", "install", "-e", ".", "--force-reinstall"])
            os.chdir(original_dir)
            print_success("seesea 包更新成功")
            return True
        except Exception as e:
            os.chdir(original_dir)  # 确保恢复目录
            print_error(f"seesea 包更新失败: {e}")
            return False

    try:
        print_info("从本地安装 seesea...")
        original_dir = os.getcwd()
        os.chdir(seesea_dir)
        run_command([sys.executable, "-m", "pip", "install", "-e", ".", "--force-reinstall"])
        os.chdir(original_dir)
        print_success("seesea 包安装成功")
        return True
    except Exception as e:
        os.chdir(original_dir)  # 确保恢复目录
        print_error(f"seesea 包安装失败: {e}")
        return False


def verify_installation():
    """验证安装是否成功"""
    print_info("验证安装...")

    try:
        # 检查是否可以导入 seesea
        result = run_command([sys.executable, "-c", "import seesea; print('seesea 导入成功')"],
                           capture_output=True)
        if result.returncode == 0:
            print_success("seesea 导入测试通过")
        else:
            print_error("seesea 导入测试失败")
            return False

        # 检查 CLI 命令是否可用
        result = run_command(["seesea", "--help"], check=False, capture_output=True)
        if result.returncode == 0:
            print_success("seesea CLI 命令可用")
        else:
            print_warning("seesea CLI 命令不可用，但这可能不影响核心功能")

        return True
    except Exception as e:
        print_error(f"安装验证失败: {e}")
        return False


def clean_build_artifacts():
    """清理构建产物（可选）"""
    if ask_user_question("是否清理旧的构建产物？"):
        script_dir = os.path.dirname(os.path.abspath(__file__))
        target_dir = os.path.join(script_dir, "target")

        try:
            if os.path.exists(target_dir):
                shutil.rmtree(target_dir)
                print_success("已清理构建产物")
            else:
                print_info("没有找到需要清理的构建产物")
        except Exception as e:
            print_error(f"清理构建产物失败: {e}")


def main():
    """主函数"""
    print(f"{Colors.BOLD}{Colors.BLUE}SeeSea 智能安装脚本{Colors.END}")
    print("=" * 50)

    # 检查 Python 版本
    if sys.version_info < (3, 8):
        print_error("需要 Python 3.8 或更高版本")
        sys.exit(1)

    print_success(f"Python 版本: {sys.version}")

    # 检查虚拟环境
    if check_venv_environment():
        print_success("检测到虚拟环境")
    else:
        print_warning("未检测到虚拟环境")
        print_warning("全局安装可能会污染 Python 环境")
        if not ask_user_question("是否继续安装？"):
            print_info("安装已取消")
            sys.exit(0)

    # 添加快速安装选项
    print_info("检测是否需要重新构建...")
    need_rebuild_flag = need_rebuild()
    if need_rebuild_flag:
        print_warning("检测到源文件变化，需要重新构建")
    else:
        print_success("源文件未变化，可跳过构建步骤")

    try:
        # 检查并安装依赖
        check_and_install_dependencies()

        # 配置 Playwright（智能跳过）
        if not setup_playwright():
            sys.exit(1)

        # 构建 Maturin 项目（智能跳过）
        if not build_with_maturin():
            sys.exit(1)

        # 安装 .whl 文件（智能跳过）
        if not install_whl_file():
            sys.exit(1)

        # 安装 seesea 包（智能跳过）
        if not install_seesea_package():
            sys.exit(1)

        # 验证安装
        if verify_installation():
            print_success(f"{Colors.BOLD}🎉 SeeSea 安装完成！{Colors.END}")
            print_info("现在可以使用 'seesea --help' 查看帮助信息")

            # 提示清理选项
            print_info("\n💡 提示：安装脚本会自动检测变化，避免重复构建")
            clean_build_artifacts()
        else:
            print_error("安装验证失败，请检查错误信息")
            sys.exit(1)

    except KeyboardInterrupt:
        print_warning("\n安装被用户中断")
        sys.exit(1)
    except Exception as e:
        print_error(f"安装过程中发生错误: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
