#!/usr/bin/env python3
"""
SeeSea 安装脚本
自动检查和安装所需依赖，构建并安装 SeeSea 项目
"""

import os
import sys
import subprocess
import importlib
import shutil
import hashlib
import json
from typing import List, Optional, Any


class Colors:
    """终端颜色输出"""

    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    RED = "\033[91m"
    BLUE = "\033[94m"
    BOLD = "\033[1m"
    END = "\033[0m"


class Logger:
    """日志记录器"""

    @staticmethod
    def info(message: str) -> None:
        """打印信息消息"""
        print(f"{Colors.BLUE}[INFO]{Colors.END} {message}")

    @staticmethod
    def success(message: str) -> None:
        """打印成功消息"""
        print(f"{Colors.GREEN}[SUCCESS]{Colors.END} {message}")

    @staticmethod
    def warning(message: str) -> None:
        """打印警告消息"""
        print(f"{Colors.YELLOW}[WARNING]{Colors.END} {message}")

    @staticmethod
    def error(message: str) -> None:
        """打印错误消息"""
        print(f"{Colors.RED}[ERROR]{Colors.END} {message}")

    @staticmethod
    def debug(message: str) -> None:
        """打印调试消息"""
        if os.environ.get("SEESEA_DEBUG") == "1":
            print(f"{Colors.BLUE}[DEBUG]{Colors.END} {message}")


class CommandRunner:
    """命令执行器"""

    @staticmethod
    def run(
        command: List[str], check: bool = True, capture_output: bool = False
    ) -> subprocess.CompletedProcess[Any]:
        """运行命令"""
        Logger.debug(f"执行命令: {' '.join(command)}")

        try:
            result = subprocess.run(
                command, 
                check=check, 
                capture_output=capture_output, 
                text=True
            )

            Logger.debug(f"命令执行成功: {' '.join(command)}")
            return result
        except subprocess.CalledProcessError as e:
            if check:
                Logger.error(f"命令执行失败: {' '.join(command)}")
                Logger.error(f"错误信息: {e}")
                if capture_output:
                    Logger.error(f"标准输出: {e.stdout}")
                    Logger.error(f"标准错误: {e.stderr}")
                sys.exit(1)
            # 创建一个CompletedProcess对象返回
            return subprocess.CompletedProcess(
                args=command,
                returncode=e.returncode,
                stdout=e.stdout if hasattr(e, 'stdout') else b'',
                stderr=e.stderr if hasattr(e, 'stderr') else b''
            )
        except Exception as e:
            if check:
                Logger.error(f"命令执行异常: {' '.join(command)}")
                Logger.error(f"异常信息: {e}")
                sys.exit(1)
            # 对于其他异常，返回一个带有错误码的CompletedProcess对象
            return subprocess.CompletedProcess(
                args=command,
                returncode=1,
                stdout=b'',
                stderr=str(e).encode()
            )
            raise e


class PackageManager:
    """包管理器"""

    @staticmethod
    def is_installed(package_name: str) -> bool:
        """检查包是否已安装"""
        try:
            importlib.import_module(package_name)
            Logger.debug(f"包 {package_name} 已安装（通过导入检查）")
            return True
        except ImportError:
            # 对于特殊的包名，使用 pip show 检查
            try:
                result = CommandRunner.run(
                    [sys.executable, "-m", "pip", "show", package_name],
                    check=False,
                    capture_output=True,
                )
                is_installed = result.returncode == 0
                Logger.debug(
                    f"包 {package_name} {'已安装' if is_installed else '未安装'}（通过 pip show 检查）"
                )
                return is_installed
            except Exception as e:
                Logger.debug(f"检查包 {package_name} 安装状态失败: {e}")
                return False

    @staticmethod
    def is_installed_with_version(package_name: str, version: str) -> bool:
        """检查包是否已安装指定版本"""
        try:
            result = CommandRunner.run(
                [sys.executable, "-m", "pip", "show", package_name],
                check=False,
                capture_output=True,
            )

            if result.returncode != 0:
                return False

            for line in result.stdout.split("\n"):
                if line.startswith("Version:"):
                    installed_version = line.split(":")[1].strip()
                    is_match = installed_version == version
                    Logger.debug(
                        f"包 {package_name} 版本 {installed_version} {'匹配' if is_match else '不匹配'}预期版本 {version}"
                    )
                    return is_match
            return False
        except Exception as e:
            Logger.debug(f"检查包 {package_name} 版本失败: {e}")
            return False

    @staticmethod
    def install(package_name: str, import_name: Optional[str] = None) -> bool:
        """安装包"""
        if import_name is None:
            import_name = package_name

        if PackageManager.is_installed(import_name):
            Logger.success(f"{package_name} 已安装")
            return True

        Logger.info(f"正在安装 {package_name}...")
        try:
            CommandRunner.run([sys.executable, "-m", "pip", "install", package_name])
            Logger.success(f"{package_name} 安装成功")
            return True
        except Exception as e:
            Logger.error(f"{package_name} 安装失败: {e}")
            return False


class BuildStateManager:
    """构建状态管理器"""

    def __init__(self):
        self.script_dir = os.path.dirname(os.path.abspath(__file__))
        self.state_file = os.path.join(self.script_dir, ".build_state")
        self.cargo_file = os.path.join(self.script_dir, "Cargo.toml")
        self.pyproject_file = os.path.join(self.script_dir, "pyproject.toml")

    def get_file_hash(self, file_path: str) -> str:
        """计算文件的哈希值"""
        if not os.path.exists(file_path):
            return ""

        hash_md5 = hashlib.md5()
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                hash_md5.update(chunk)
        return hash_md5.hexdigest()

    def load_state(self) -> dict:
        """加载构建状态"""
        if os.path.exists(self.state_file):
            try:
                with open(self.state_file, "r") as f:
                    content = f.read().strip()
                    if content:
                        state = json.loads(content)
                        Logger.debug(f"加载构建状态: {state}")
                        return state
            except (json.JSONDecodeError, Exception) as e:
                Logger.debug(f"加载构建状态失败: {e}")
        return {}

    def save_state(self, state: dict) -> None:
        """保存构建状态"""
        Logger.debug(f"保存构建状态: {state}")
        with open(self.state_file, "w") as f:
            json.dump(state, f)

    def need_rebuild(self) -> bool:
        """检查是否需要重新构建"""
        # 检查源文件是否存在
        if not os.path.exists(self.cargo_file) or not os.path.exists(self.pyproject_file):
            Logger.debug("源文件不存在，需要重新构建")
            return True

        # 计算当前文件哈希
        current_hashes = {
            "cargo": self.get_file_hash(self.cargo_file),
            "pyproject": self.get_file_hash(self.pyproject_file),
        }

        # 加载历史哈希
        state = self.load_state()
        history_hashes = state.get("hashes", {})

        # 如果哈希值变化，需要重新构建
        if current_hashes != history_hashes:
            Logger.debug(f"源文件哈希变化，需要重新构建: {current_hashes} != {history_hashes}")
            return True

        # 检查是否有wheel文件
        target_dir = os.path.join(self.script_dir, "target", "wheels")
        if os.path.exists(target_dir):
            whl_files = [f for f in os.listdir(target_dir) if f.endswith(".whl")]
            if whl_files:
                # 选择最新的wheel文件
                latest_whl = sorted(whl_files)[-1]
                whl_path = os.path.join(target_dir, latest_whl)

                # 检查wheel文件是否比源文件新
                whl_mtime = os.path.getmtime(whl_path)
                cargo_mtime = os.path.getmtime(self.cargo_file)
                pyproject_mtime = os.path.getmtime(self.pyproject_file)

                if whl_mtime > max(cargo_mtime, pyproject_mtime):
                    Logger.debug(f"wheel文件 {latest_whl} 比源文件新，不需要重新构建")
                    return False

        Logger.debug("需要重新构建")
        return True

    def update_state(self) -> None:
        """更新构建状态"""
        state = self.load_state()
        state["hashes"] = {
            "cargo": self.get_file_hash(self.cargo_file),
            "pyproject": self.get_file_hash(self.pyproject_file),
        }
        self.save_state(state)


class EnvironmentChecker:
    """环境检查器"""

    @staticmethod
    def is_in_venv() -> bool:
        """检查是否在虚拟环境中"""
        # 检查标准的虚拟环境指示器
        if hasattr(sys, "real_prefix") or (
            hasattr(sys, "base_prefix") and sys.base_prefix != sys.prefix
        ):
            return True

        # 检查 VIRTUAL_ENV 环境变量
        if os.environ.get("VIRTUAL_ENV"):
            return True

        # 检查 conda 环境
        if os.environ.get("CONDA_DEFAULT_ENV") and os.environ.get("CONDA_PREFIX"):
            return True

        return False

    @staticmethod
    def check_python_version() -> bool:
        """检查 Python 版本"""
        if sys.version_info < (3, 8):
            Logger.error("需要 Python 3.8 或更高版本")
            return False

        Logger.success(f"Python 版本: {sys.version}")
        return True


class UserInteractor:
    """用户交互器"""

    @staticmethod
    def ask_yes_no(question: str, default: bool = True) -> bool:
        """询问用户是/否问题"""
        try:
            while True:
                default_str = "y" if default else "n"
                response = (
                    input(f"{Colors.YELLOW}{question} (y/n, 默认 {default_str}): {Colors.END}")
                    .strip()
                    .lower()
                )

                if not response:
                    return default
                elif response in ["y", "yes", "是"]:
                    return True
                elif response in ["n", "no", "否"]:
                    return False
                else:
                    Logger.warning("请输入 'y' 或 'n'")
        except (EOFError, KeyboardInterrupt):
            # 在非交互环境或用户中断时返回默认值
            Logger.warning("检测到非交互环境或用户中断，使用默认选项")
            return default


class SeeSeaInstaller:
    """SeeSea 安装器"""

    def __init__(self):
        self.script_dir = os.path.dirname(os.path.abspath(__file__))
        self.build_state = BuildStateManager()
        self.need_rebuild = self.build_state.need_rebuild()

    def check_and_install_dependencies(self) -> None:
        """检查并安装所需依赖"""
        Logger.info("检查所需依赖...")

        required_packages = [
            ("playwright", "playwright"),
            ("maturin[patchelf]", "maturin"),
            ("rich", "rich"),
            ("click", "click"),
            ("typing-extensions", "typing_extensions"),
        ]

        missing_packages = []

        # 检查每个包
        for package_name, import_name in required_packages:
            if not PackageManager.is_installed(import_name):
                missing_packages.append((package_name, import_name))
            else:
                Logger.success(f"{package_name} 已安装")

        # 如果有缺失的包，询问用户是否安装
        if missing_packages:
            missing_names = [pkg[0] for pkg in missing_packages]
            Logger.warning(f"发现缺失的依赖: {missing_names}")

            if UserInteractor.ask_yes_no("是否安装缺失的依赖？", default=True):
                for package_name, import_name in missing_packages:
                    if not PackageManager.install(package_name, import_name):
                        Logger.error(f"安装 {package_name} 失败，安装中止")
                        sys.exit(1)
            else:
                Logger.error("缺少必要依赖，无法继续安装")
                sys.exit(1)

    def setup_playwright(self) -> bool:
        """安装和配置 Playwright"""
        Logger.info("配置 Playwright...")

        if not PackageManager.is_installed("playwright"):
            Logger.error("Playwright 未安装")
            return False

        try:
            # 安装 Playwright 依赖
            Logger.info("安装 Playwright 依赖...")
            CommandRunner.run([sys.executable, "-m", "playwright", "install-deps"], check=False)

            # 安装 Playwright Chromium
            Logger.info("安装 Playwright Chromium...")
            CommandRunner.run(
                [sys.executable, "-m", "playwright", "install", "chromium"], check=False
            )

            Logger.success("Playwright 配置完成")
            return True
        except Exception as e:
            Logger.error(f"Playwright 配置失败: {e}")
            return False

    def build_with_maturin(self) -> bool:
        """使用 Maturin 构建项目"""
        if not self.need_rebuild:
            Logger.success("项目已构建且为最新，跳过构建")
            return True

        if not PackageManager.is_installed("maturin"):
            Logger.error("Maturin 未安装")
            return False

        Logger.info("使用 Maturin 构建项目...")

        original_dir = os.getcwd()
        try:
            # 切换到项目根目录
            os.chdir(self.script_dir)

            # 构建项目
            Logger.info("构建项目...")
            CommandRunner.run(["maturin", "build", "--release", "--strip"])

            # 更新构建状态
            self.build_state.update_state()

            Logger.success("Maturin 构建完成")
            return True
        except Exception as e:
            Logger.error(f"Maturin 构建失败: {e}")
            return False
        finally:
            # 恢复原始目录
            os.chdir(original_dir)

    def install_whl_file(self) -> bool:
        """安装构建生成的 .whl 文件"""
        target_dir = os.path.join(self.script_dir, "target", "wheels")

        if not os.path.exists(target_dir):
            Logger.error(f"构建目标目录不存在: {target_dir}")
            return False

        # 查找 .whl 文件
        whl_files = [f for f in os.listdir(target_dir) if f.endswith(".whl")]

        if not whl_files:
            Logger.error("未找到 .whl 文件")
            return False

        # 选择最新的 .whl 文件
        whl_file = sorted(whl_files)[-1]
        whl_path = os.path.join(target_dir, whl_file)

        # 从文件名提取包名和版本
        try:
            base_name = whl_file.replace(".whl", "")
            parts = base_name.split("-")
            if len(parts) >= 2:
                package_name = parts[0]
                version = parts[1]

                # 检查是否已安装相同版本
                if PackageManager.is_installed_with_version(package_name, version):
                    Logger.success(f"{package_name} 版本 {version} 已安装，跳过")
                    return True
        except Exception as e:
            Logger.debug(f"解析 .whl 文件名失败: {e}")

        try:
            Logger.info(f"安装 {whl_file}...")
            CommandRunner.run(
                [sys.executable, "-m", "pip", "install", whl_path, "--force-reinstall"]
            )
            Logger.success(f"{whl_file} 安装成功")
            return True
        except Exception as e:
            Logger.error(f"安装 .whl 文件失败: {e}")
            return False

    def install_seesea_package(self) -> bool:
        """安装 seesea 包"""
        seesea_dir = os.path.join(self.script_dir, "seesea")

        if not os.path.exists(seesea_dir):
            Logger.error(f"seesea 目录不存在: {seesea_dir}")
            return False

        original_dir = os.getcwd()
        try:
            os.chdir(seesea_dir)

            # 安装 seesea 包
            Logger.info("安装 seesea 包...")
            CommandRunner.run(
                [sys.executable, "-m", "pip", "install", "-e", ".", "--force-reinstall"]
            )

            Logger.success("seesea 包安装成功")
            return True
        except Exception as e:
            Logger.error(f"seesea 包安装失败: {e}")
            return False
        finally:
            os.chdir(original_dir)

    def verify_installation(self) -> bool:
        """验证安装是否成功"""
        Logger.info("验证安装...")

        try:
            # 检查是否可以导入 seesea
            result = CommandRunner.run(
                [sys.executable, "-c", "import seesea; print('seesea 导入成功')"],
                capture_output=True,
            )
            if result.returncode == 0:
                Logger.success("seesea 导入测试通过")
            else:
                Logger.error("seesea 导入测试失败")
                return False

            # 检查 CLI 命令是否可用
            result = CommandRunner.run(["seesea", "--help"], check=False, capture_output=True)
            if result.returncode == 0:
                Logger.success("seesea CLI 命令可用")
            else:
                Logger.warning("seesea CLI 命令不可用，但这可能不影响核心功能")

            return True
        except Exception as e:
            Logger.error(f"安装验证失败: {e}")
            return False

    def clean_build_artifacts(self) -> None:
        """清理构建产物"""
        if UserInteractor.ask_yes_no("是否清理旧的构建产物？", default=False):
            target_dir = os.path.join(self.script_dir, "target")

            try:
                if os.path.exists(target_dir):
                    shutil.rmtree(target_dir)
                    Logger.success("已清理构建产物")
                else:
                    Logger.info("没有找到需要清理的构建产物")
            except Exception as e:
                Logger.error(f"清理构建产物失败: {e}")

    def run(self) -> None:
        """运行安装流程"""
        Logger.info(f"{Colors.BOLD}SeeSea 智能安装脚本{Colors.END}")
        Logger.info("=" * 50)

        # 检查 Python 版本
        if not EnvironmentChecker.check_python_version():
            sys.exit(1)

        # 检查虚拟环境
        if EnvironmentChecker.is_in_venv():
            Logger.success("检测到虚拟环境")
        else:
            Logger.warning("未检测到虚拟环境")
            Logger.warning("全局安装可能会污染 Python 环境")
            if not UserInteractor.ask_yes_no("是否继续安装？", default=True):
                Logger.info("安装已取消")
                sys.exit(0)

        # 显示构建状态
        if self.need_rebuild:
            Logger.warning("检测到源文件变化，需要重新构建")
        else:
            Logger.success("源文件未变化，可跳过构建步骤")

        try:
            # 检查并安装依赖
            self.check_and_install_dependencies()

            # 配置 Playwright
            if not self.setup_playwright():
                sys.exit(1)

            # 构建 Maturin 项目
            if not self.build_with_maturin():
                sys.exit(1)

            # 安装 .whl 文件
            if not self.install_whl_file():
                sys.exit(1)

            # 安装 seesea 包
            if not self.install_seesea_package():
                sys.exit(1)

            # 验证安装
            if self.verify_installation():
                Logger.success(f"{Colors.BOLD}🎉 SeeSea 安装完成！{Colors.END}")
                Logger.info("现在可以使用 'seesea --help' 查看帮助信息")

                # 提示清理选项
                Logger.info("\n💡 提示：安装脚本会自动检测变化，避免重复构建")
                self.clean_build_artifacts()
            else:
                Logger.error("安装验证失败，请检查错误信息")
                sys.exit(1)
        except KeyboardInterrupt:
            Logger.warning("\n安装被用户中断")
            sys.exit(1)
        except Exception as e:
            Logger.error(f"安装过程中发生错误: {e}")
            import traceback

            traceback.print_exc()
            sys.exit(1)


def main():
    """主函数"""
    installer = SeeSeaInstaller()
    installer.run()


if __name__ == "__main__":
    main()
