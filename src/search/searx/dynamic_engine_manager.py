# SPDX-License-Identifier: AGPL-3.0-or-later
"""Dynamic Engine Manager - 简化版，仅用于全局模式下的引擎动态管理"""

import json
import time
import threading
from typing import Dict, Set, Optional
from pathlib import Path

from searx import logger, settings
from searx.engines import engines

logger = logger.getChild('dynamic_engine_manager')

# 全局引擎失败记录
_engine_failures: Dict[str, Dict] = {}
_disabled_engines: Set[str] = set()
_lock = threading.Lock()

# 关键状态码 - 非200状态码会导致引擎被禁用
FAILURE_STATUS_CODES = {
    403,  # Forbidden
    404,  # Not Found
    408,  # Request Timeout
    429,  # Too Many Requests
    500,  # Internal Server Error
    502,  # Bad Gateway
    503,  # Service Unavailable
    504,  # Gateway Timeout
}

# 配置参数
FAILURE_THRESHOLD = 1  # 全局模式下，一次失败就禁用
MAX_FAILURE_AGE = 3600  # 1小时后重新考虑启用

# 状态文件路径
STATE_FILE = Path('/tmp/engine_state.json')


def record_engine_failure(engine_name: str, status_code: int, reason: str = ""):
    """记录引擎失败并禁用引擎（仅限全局模式）"""

    # 只处理关键状态码
    if status_code not in FAILURE_STATUS_CODES:
        return

    with _lock:
        _disabled_engines.add(engine_name)

        # 记录失败信息
        if engine_name not in _engine_failures:
            _engine_failures[engine_name] = {
                'count': 0,
                'first_failure': 0,
                'last_failure': 0,
                'status_codes': []
            }

        failure_info = _engine_failures[engine_name]
        failure_info['count'] += 1
        failure_info['last_failure'] = time.time()

        if failure_info['first_failure'] == 0:
            failure_info['first_failure'] = failure_info['last_failure']

        if status_code not in failure_info['status_codes']:
            failure_info['status_codes'].append(status_code)

        logger.info(f"🚫 禁用引擎 {engine_name}: HTTP {status_code} - {reason}")

        # 立即从可用引擎中移除
        _remove_engine_from_active_list(engine_name)

        save_state()


def _remove_engine_from_active_list(engine_name: str):
    """从活跃引擎列表中移除引擎"""
    if engine_name in engines:
        # 设置为disabled而不是删除，避免破坏系统稳定性
        engines[engine_name].disabled = True

        # 从分类中移除
        from searx.engines import categories
        for category_name, category_engines in categories.items():
            if engine_name in category_engines:
                category_engines.remove(engine_name)


def should_disable_engine(engine_name: str, status_code: int) -> bool:
    """检查是否应该禁用引擎"""
    return status_code in FAILURE_STATUS_CODES


def get_engine_status() -> Dict:
    """获取引擎状态"""
    with _lock:
        return {
            'disabled_engines': list(_disabled_engines),
            'engine_failures': dict(_engine_failures),
            'total_engines': len(engines),
            'active_engines': len([e for e in engines.values() if not e.disabled]),
            'timestamp': time.time()
        }


def save_state():
    """保存引擎状态"""
    try:
        state_data = {
            'disabled_engines': list(_disabled_engines),
            'engine_failures': _engine_failures,
            'timestamp': time.time()
        }

        STATE_FILE.parent.mkdir(parents=True, exist_ok=True)

        with open(STATE_FILE, 'w', encoding='utf-8') as f:
            json.dump(state_data, f, indent=2, ensure_ascii=False)

    except Exception as e:
        logger.error(f"保存引擎状态失败: {e}")


def load_state():
    """加载引擎状态"""
    try:
        if STATE_FILE.exists():
            with open(STATE_FILE, 'r', encoding='utf-8') as f:
                state_data = json.load(f)

            global _disabled_engines, _engine_failures
            _disabled_engines = set(state_data.get('disabled_engines', []))
            _engine_failures = state_data.get('engine_failures', {})

            # 应用禁用状态
            for engine_name in _disabled_engines:
                if engine_name in engines:
                    engines[engine_name].disabled = True
                    _remove_engine_from_active_list(engine_name)

            logger.info(f"加载引擎状态: {len(_disabled_engines)} 个已禁用引擎")
            return True
    except Exception as e:
        logger.error(f"加载引擎状态失败: {e}")

    return False


def reenable_all_engines():
    """重新启用所有引擎"""
    global _disabled_engines, _engine_failures

    logger.info("🔄 重新启用所有引擎...")

    with _lock:
        cleared_count = 0
        for engine_name in list(_disabled_engines):
            _disabled_engines.discard(engine_name)
            if engine_name in engines:
                engines[engine_name].disabled = False
                cleared_count += 1

        # 清空失败记录
        _engine_failures.clear()

        # 重新构建分类列表
        from searx.engines import categories
        for category_name, category_engines in categories.items():
            category_engines.clear()

        for engine in engines.values():
            if not engine.disabled:
                for category_name in engine.categories:
                    categories.setdefault(category_name, []).append(engine)

        logger.info(f"✅ 重新启用 {cleared_count} 个引擎")
        save_state()


def check_and_reenable_engines():
    """检查并重新启用超时的引擎"""
    with _lock:
        current_time = time.time()
        reenabled = []

        for engine_name in list(_disabled_engines):
            if engine_name in _engine_failures:
                failure_info = _engine_failures[engine_name]
                if current_time - failure_info['last_failure'] > MAX_FAILURE_AGE:
                    _disabled_engines.discard(engine_name)
                    if engine_name in engines:
                        engines[engine_name].disabled = False
                        failure_info['count'] = 0
                        failure_info['status_codes'] = []
                        reenabled.append(engine_name)
                        logger.info(f"重新启用引擎: {engine_name}")

        if reenabled:
            save_state()

        return reenabled


def initialize():
    """初始化动态引擎管理器"""
    if settings.get('engine_loading_mode') != 'global':
        logger.info("⚙️ 设置模式 - 动态引擎管理器已禁用")
        return

    logger.info("🌍 全局模式 - 启动动态引擎管理器")

    # 加载之前的状态
    load_state()

    # 启动后台检查线程
    def background_check():
        while True:
            try:
                check_and_reenable_engines()
                time.sleep(300)  # 每5分钟检查一次
            except Exception as e:
                logger.error(f"后台引擎检查错误: {e}")
                time.sleep(60)

    thread = threading.Thread(target=background_check, daemon=True, name="DynamicEngineManager")
    thread.start()

    logger.info("✅ 动态引擎管理器初始化完成")