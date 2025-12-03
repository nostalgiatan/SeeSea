// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Python bindings for configuration

use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyConfig {
    #[pyo3(get, set)]
    pub debug: bool,
    #[pyo3(get, set)]
    pub max_results: usize,
    #[pyo3(get, set)]
    pub timeout_seconds: u64,
}

#[pymethods]
impl PyConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            debug: false,
            max_results: 100,
            timeout_seconds: 30,
        }
    }
}
