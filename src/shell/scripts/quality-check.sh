#!/bin/bash
# Quality check script - Run all code quality checks locally
# This is the same set of checks that run in CI/CD

set -e

echo "🔍 Running code quality checks..."
echo ""

echo "1️⃣ Formatting with Black..."
uv run black --check agfs_shell/ tests/
echo "✅ Black check passed"
echo ""

echo "2️⃣ Sorting imports with isort..."
uv run isort --check-only agfs_shell/ tests/
echo "✅ isort check passed"
echo ""

echo "3️⃣ Linting with ruff..."
uv run ruff check agfs_shell/ tests/
echo "✅ ruff check passed"
echo ""

echo "4️⃣ Running tests..."
uv run pytest tests/ -q
echo "✅ Tests passed"
echo ""

echo "5️⃣ Coverage report..."
uv run pytest tests/ --cov=agfs_shell --cov-report=term-missing | tail -20
echo ""

echo "🎉 All quality checks passed!"
