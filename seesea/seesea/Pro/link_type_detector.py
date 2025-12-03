# Copyright (C) 2025 nostalgiatan
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

"""
链接类型检测器

该模块用于判定链接网页的类型（HTML或SPA），并包含一个特殊化字典，用于处理特定域名的特殊情况。

例如，新华网虽然是SPA应用，但网页正文可以由HTML的常规流程解析，因此需要特殊化处理为HTML类型。

示例用法：
    >>> from seesea.Pro.link_type_detector import detect_link_type
    >>> detect_link_type("https://www.news.cn/")
    'html'
    >>> detect_link_type("https://www.example.com/")
    'spa'
"""

from typing import Dict

# 特殊化字典：域名前缀 -> 页面类型
# 用于处理特殊情况，比如某些SPA应用的正文可以通过HTML常规流程解析
SPECIALIZED_DOMAINS: Dict[str, str] = {
    # 新华网相关域名，特殊处理为HTML类型
    "https://www.news.cn": "html",
    "https://news.cn": "html",
    "https://www.xinhuanet.com": "html",
    "https://xinhuanet.com": "html",
    # 可以根据需要添加更多特殊域名
}


def detect_link_type(url: str) -> str:
    """
    判定链接的网页类型

    参数:
        url: 链接URL

    返回:
        str: 页面类型，"html"或"spa"
    """
    # 1. 检查是否在特殊化字典中
    for domain_prefix, page_type in SPECIALIZED_DOMAINS.items():
        if url.startswith(domain_prefix):
            return page_type

    # 2. 其他判定逻辑可以在这里添加
    # 例如：基于URL路径、查询参数等特征的判定

    # 默认返回SPA，因为现代网站大多是SPA
    return "spa"


def add_specialized_domain(domain_prefix: str, page_type: str) -> None:
    """
    添加特殊化域名规则

    参数:
        domain_prefix: 域名前缀，如"https://www.news.cn"
        page_type: 页面类型，"html"或"spa"
    """
    SPECIALIZED_DOMAINS[domain_prefix] = page_type


def remove_specialized_domain(domain_prefix: str) -> None:
    """
    移除特殊化域名规则

    参数:
        domain_prefix: 域名前缀，如"https://www.news.cn"
    """
    SPECIALIZED_DOMAINS.pop(domain_prefix, None)


def get_specialized_domains() -> Dict[str, str]:
    """
    获取所有特殊化域名规则

    返回:
        Dict[str, str]: 特殊化域名规则字典的副本
    """
    return SPECIALIZED_DOMAINS.copy()
