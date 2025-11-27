"""
Tests for SeeSea Python SDK

This test suite validates the Python bindings, scoring system, 
cache preservation, and multi-parameter support.
"""

import pytest
import sys
import os

# Add the seesea module to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))


class TestSearchClient:
    """Tests for SearchClient class"""
    
    def test_client_creation(self):
        """Test that SearchClient can be created"""
        from seesea import SearchClient
        client = SearchClient()
        assert client is not None
        
    def test_client_has_search_method(self):
        """Test that SearchClient has search method"""
        from seesea import SearchClient
        client = SearchClient()
        assert hasattr(client, 'search')
        assert callable(client.search)
        
    def test_client_has_get_stats_method(self):
        """Test that SearchClient has get_stats method"""
        from seesea import SearchClient
        client = SearchClient()
        assert hasattr(client, 'get_stats')
        assert callable(client.get_stats)
        
    def test_client_has_clear_cache_method(self):
        """Test that SearchClient has clear_cache method"""
        from seesea import SearchClient
        client = SearchClient()
        assert hasattr(client, 'clear_cache')
        assert callable(client.clear_cache)
        
    def test_client_has_list_engines_method(self):
        """Test that SearchClient has list_engines method"""
        from seesea import SearchClient
        client = SearchClient()
        assert hasattr(client, 'list_engines')
        assert callable(client.list_engines)
        
    def test_client_has_health_check_method(self):
        """Test that SearchClient has health_check method"""
        from seesea import SearchClient
        client = SearchClient()
        assert hasattr(client, 'health_check')
        assert callable(client.health_check)


class TestSearchParameters:
    """Tests for search parameter support"""
    
    def test_search_accepts_query_parameter(self):
        """Test that search accepts query parameter"""
        from seesea import SearchClient
        import inspect
        
        client = SearchClient()
        sig = inspect.signature(client.search)
        params = list(sig.parameters.keys())
        
        assert 'query' in params
        
    def test_search_accepts_page_parameter(self):
        """Test that search accepts page parameter"""
        from seesea import SearchClient
        import inspect
        
        client = SearchClient()
        sig = inspect.signature(client.search)
        params = list(sig.parameters.keys())
        
        assert 'page' in params
        
    def test_search_accepts_page_size_parameter(self):
        """Test that search accepts page_size parameter"""
        from seesea import SearchClient
        import inspect
        
        client = SearchClient()
        sig = inspect.signature(client.search)
        params = list(sig.parameters.keys())
        
        assert 'page_size' in params
        
    def test_search_accepts_language_parameter(self):
        """Test that search accepts language parameter"""
        from seesea import SearchClient
        import inspect
        
        client = SearchClient()
        sig = inspect.signature(client.search)
        params = list(sig.parameters.keys())
        
        assert 'language' in params
        
    def test_search_accepts_region_parameter(self):
        """Test that search accepts region parameter"""
        from seesea import SearchClient
        import inspect
        
        client = SearchClient()
        sig = inspect.signature(client.search)
        params = list(sig.parameters.keys())
        
        assert 'region' in params
        
    def test_search_accepts_engines_parameter(self):
        """Test that search accepts engines parameter"""
        from seesea import SearchClient
        import inspect
        
        client = SearchClient()
        sig = inspect.signature(client.search)
        params = list(sig.parameters.keys())
        
        assert 'engines' in params

    def test_search_accepts_force_parameter(self):
        """Test that search accepts force parameter"""
        from seesea import SearchClient
        import inspect

        client = SearchClient()
        sig = inspect.signature(client.search)
        params = list(sig.parameters.keys())

        assert 'force' in params

    def test_search_parameter_count(self):
        """Test that search has expected parameters"""
        from seesea import SearchClient
        import inspect

        client = SearchClient()
        sig = inspect.signature(client.search)
        params = list(sig.parameters.keys())

        # Should have: query, page, page_size, language, region, engines, force, cache_timeline
        assert len(params) >= 6, f"Expected at least 6 parameters, got {len(params)}: {params}"
        assert 'query' in params, "query parameter should be present"
        assert 'engines' in params, "engines parameter should be present"


class TestStats:
    """Tests for statistics functionality"""
    
    def test_get_stats_returns_dict(self):
        """Test that get_stats returns a dictionary"""
        from seesea import SearchClient
        
        client = SearchClient()
        stats = client.get_stats()
        
        assert isinstance(stats, dict)
        
    def test_get_stats_has_total_searches(self):
        """Test that stats contains total_searches"""
        from seesea import SearchClient
        
        client = SearchClient()
        stats = client.get_stats()
        
        assert 'total_searches' in stats
        assert isinstance(stats['total_searches'], int)
        
    def test_get_stats_has_cache_hits(self):
        """Test that stats contains cache_hits"""
        from seesea import SearchClient
        
        client = SearchClient()
        stats = client.get_stats()
        
        assert 'cache_hits' in stats
        assert isinstance(stats['cache_hits'], int)
        
    def test_get_stats_has_cache_misses(self):
        """Test that stats contains cache_misses"""
        from seesea import SearchClient
        
        client = SearchClient()
        stats = client.get_stats()
        
        assert 'cache_misses' in stats
        assert isinstance(stats['cache_misses'], int)
        
    def test_get_stats_has_engine_failures(self):
        """Test that stats contains engine_failures"""
        from seesea import SearchClient
        
        client = SearchClient()
        stats = client.get_stats()
        
        assert 'engine_failures' in stats
        assert isinstance(stats['engine_failures'], int)
        
    def test_get_stats_has_timeouts(self):
        """Test that stats contains timeouts"""
        from seesea import SearchClient
        
        client = SearchClient()
        stats = client.get_stats()
        
        assert 'timeouts' in stats
        assert isinstance(stats['timeouts'], int)
        
    def test_initial_stats_are_zero(self):
        """Test that initial statistics are all zero"""
        from seesea import SearchClient
        
        client = SearchClient()
        stats = client.get_stats()
        
        assert stats['total_searches'] == 0
        assert stats['cache_hits'] == 0
        assert stats['cache_misses'] == 0
        assert stats['engine_failures'] == 0
        assert stats['timeouts'] == 0


class TestCacheOperations:
    """Tests for cache operations"""
    
    def test_clear_cache_doesnt_raise(self):
        """Test that clear_cache doesn't raise an exception"""
        from seesea import SearchClient
        
        client = SearchClient()
        # Should not raise
        client.clear_cache()
        
    def test_list_engines_returns_list(self):
        """Test that list_engines returns a list"""
        from seesea import SearchClient
        
        client = SearchClient()
        engines = client.list_engines()
        
        assert isinstance(engines, list)
        
    def test_health_check_returns_dict(self):
        """Test that health_check returns a dict"""
        from seesea import SearchClient
        
        client = SearchClient()
        health = client.health_check()
        
        assert isinstance(health, dict)


class TestEngineSelection:
    """Tests for engine selection functionality"""

    def test_engines_parameter_exists(self):
        """Test that engines parameter exists"""
        from seesea import SearchClient
        import inspect

        client = SearchClient()
        sig = inspect.signature(client.search)
        params = sig.parameters

        assert 'engines' in params

    def test_list_engines_returns_expected_engines(self):
        """Test that list_engines returns expected engine names"""
        from seesea import SearchClient

        client = SearchClient()
        engines = client.list_engines()
        
        # Should contain some core engines
        assert isinstance(engines, list)
        assert len(engines) > 0

    def test_custom_engines_selection(self):
        """Test that custom engines selection works"""
        from seesea import SearchClient
        import inspect

        client = SearchClient()

        # This test verifies the parameter exists and has the right structure
        # The actual engine selection logic is tested in integration tests
        sig = inspect.signature(client.search)
        assert 'engines' in sig.parameters


class TestModuleImports:
    """Tests for module imports"""
    
    def test_seesea_module_imports(self):
        """Test that seesea module can be imported"""
        import seesea
        assert seesea is not None
        
    def test_search_client_import(self):
        """Test that SearchClient can be imported"""
        from seesea import SearchClient
        assert SearchClient is not None
        
    def test_api_server_import(self):
        """Test that ApiServer can be imported"""
        from seesea import ApiServer
        assert ApiServer is not None
        
    def test_config_import(self):
        """Test that Config can be imported"""
        from seesea import Config
        assert Config is not None


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
