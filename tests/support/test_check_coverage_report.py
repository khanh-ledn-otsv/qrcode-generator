import unittest

from check_coverage_report import CoverageError, coverage_for_scope


class CoverageReportTests(unittest.TestCase):
    def test_scope_aggregates_only_named_files(self) -> None:
        report = {
            "data": [
                {
                    "files": [
                        {
                            "filename": "/workspace/src/matrix.rs",
                            "summary": {
                                "lines": {"count": 100, "covered": 98},
                                "regions": {"count": 80, "covered": 76},
                            },
                        },
                        {
                            "filename": "/workspace/src/web.rs",
                            "summary": {
                                "lines": {"count": 100, "covered": 0},
                                "regions": {"count": 100, "covered": 0},
                            },
                        },
                    ]
                }
            ]
        }

        coverage = coverage_for_scope(report, ("src/matrix.rs",))

        self.assertEqual((coverage.line_percent, coverage.region_percent), (98.0, 95.0))

    def test_missing_scope_is_an_error(self) -> None:
        with self.assertRaises(CoverageError):
            coverage_for_scope({"data": [{"files": []}]}, ("src/matrix.rs",))


if __name__ == "__main__":
    unittest.main()
