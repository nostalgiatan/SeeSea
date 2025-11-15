"""
SeeSea Utilities - 工具函数
"""

from typing import Dict, List, Any


def format_results(results: List[Dict[str, Any]], max_description_length: int = 200) -> List[Dict[str, Any]]:
    """
    格式化搜索结果
    
    Args:
        results: 原始结果列表
        max_description_length: 描述最大长度
    
    Returns:
        格式化后的结果列表
    """
    formatted = []
    for item in results:
        formatted_item = {
            'title': item.get('title', ''),
            'url': item.get('url', ''),
            'description': item.get('content', '')[:max_description_length],
            'score': item.get('score', 0.0),
        }
        formatted.append(formatted_item)
    return formatted


def parse_query(query: str) -> Dict[str, Any]:
    """
    解析查询字符串
    
    Args:
        query: 查询字符串
    
    Returns:
        解析后的查询参数
    """
    params = {'query': query.strip()}
    
    # 支持简单的过滤语法
    # 例如: "python lang:en site:github.com"
    parts = query.split()
    filters = {}
    clean_query = []
    
    for part in parts:
        if ':' in part:
            key, value = part.split(':', 1)
            if key in ['lang', 'language']:
                filters['language'] = value
            elif key == 'site':
                filters['site'] = value
        else:
            clean_query.append(part)
    
    params['query'] = ' '.join(clean_query)
    params.update(filters)
    
    return params
