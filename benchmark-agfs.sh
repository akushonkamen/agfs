#!/bin/bash
# AGFS 大规模性能测试脚本
# 对比 Go (8081) vs Rust (8080)

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[TEST]${NC} $1"; }
log_go() { echo -e "${BLUE}[GO]${NC} $1"; }
log_rust() { echo -e "${YELLOW}[RUST]${NC} $1"; }
log_error() { echo -e "${RED}[FAIL]${NC} $1"; }

# 测试数据大小
SIZES=(
    "1K:1024"
    "10K:10240"
    "100K:102400"
    "1M:1048576"
    "10M:10485760"
)

# 文件数量
FILE_COUNTS=(100 500 1000 5000)

# 生成随机数据
generate_data() {
    local size=$1
    if command -v openssl >/dev/null 2>&1; then
        openssl rand -hex $((size / 2)) 2>/dev/null | head -c $size
    else
        dd if=/dev/urandom bs=$size count=1 2>/dev/null
    fi
}

# 测试单个文件大小影响
test_file_sizes() {
    local base_url=$1
    local name=$2

    log_info "[$name] 测试不同文件大小..."

    for size_info in "${SIZES[@]}"; do
        IFS=':' read -r label bytes <<< "$size_info"

        local path="/bench-size-${label}.bin"

        # 生成数据
        local data=$(generate_data $bytes)
        local actual_size=${#data}

        # 写入测试
        local start=$(date +%s%3N)
        curl -s -X POST "${base_url}/files?path=${path}" >/dev/null 2>&1
        echo "$data" | curl -s -X PUT "${base_url}/files?path=${path}" --data-binary @- >/dev/null 2>&1
        local end=$(date +%s%3N)
        local write_time=$((end - start))

        # 读取测试
        local start=$(date +%s%3N)
        curl -s "${base_url}/files?path=${path}" >/dev/null 2>&1
        local end=$(date +%s%3N)
        local read_time=$((end - start))

        # 吞吐量计算
        local write_throughput=$((actual_size * 1000 / write_time))
        local read_throughput=$((actual_size * 1000 / read_time))

        echo "  ${label}: Write=${write_time}ms ($(($write_throughput/1024))KB/s), Read=${read_time}ms ($(($read_throughput/1024))KB/s)"

        # 清理
        curl -s -X DELETE "${base_url}/files?path=${path}" >/dev/null 2>&1
    done
}

# 测试大量文件
test_many_files() {
    local base_url=$1
    local name=$2
    local count=$3

    log_info "[$name] 测试 $count 个文件..."

    local start=$(date +%s%3N)

    # 批量创建和写入
    for i in $(seq 1 $count); do
        local path="/bench-many-$i.txt"
        curl -s -X POST "${base_url}/files?path=${path}" >/dev/null 2>&1
        echo "data-$i" | curl -s -X PUT "${base_url}/files?path=${path}" --data-binary @- >/dev/null 2>&1

        if [ $((i % 100)) -eq 0 ]; then
            echo -n "  进度: $i/$count"$'\r'
        fi
    done

    local end=$(date +%s%3N)
    local total_time=$((end - start))
    local avg_time=$((total_time / count))

    echo "  完成: $count 文件, 总耗时 ${total_time}ms, 平均 ${avg_time}ms/文件"

    # 批量读取测试
    local start=$(date +%s%3N)
    for i in $(seq 1 $count); do
        curl -s "${base_url}/files?path=/bench-many-$i.txt" >/dev/null 2>&1
    done
    local end=$(date +%s%3N)
    local read_time=$((end - start))
    local read_avg=$((read_time / count))

    echo "  读取: $count 文件, 总耗时 ${read_time}ms, 平均 ${read_avg}ms/文件"

    # 目录列表测试
    local start=$(date +%s%3N)
    curl -s "${base_url}/directories?path=/" >/dev/null 2>&1
    local end=$(date +%s%3N)
    local list_time=$((end - start))

    echo "  列表: ${list_time}ms"

    # 清理
    for i in $(seq 1 $count); do
        curl -s -X DELETE "${base_url}/files?path=/bench-many-$i.txt" >/dev/null 2>&1
    done
}

# 测试并发性能
test_concurrent() {
    local base_url=$1
    local name=$2
    local workers=$3
    local files_per_worker=$4

    local total_files=$((workers * files_per_worker))
    log_info "[$name] 并发测试: $workers 进程 x $files_per_worker 文件 = $total_files"

    local start=$(date +%s%3N)

    for w in $(seq 1 $workers); do
        (
            for i in $(seq 1 $files_per_worker); do
                local path="/conc-$w-$i.txt"
                curl -s -X POST "${base_url}/files?path=${path}" >/dev/null 2>&1
                echo "worker-$w-file-$i" | curl -s -X PUT "${base_url}/files?path=${path}" --data-binary @- >/dev/null 2>&1
            done
        ) &
    done

    wait

    local end=$(date +%s%3N)
    local total_time=$((end - start))

    echo "  写入: ${total_time}ms ($(($total_files * 1000 / total_time)) ops/sec)"

    # 并发读取
    local start=$(date +%s%3N)

    for w in $(seq 1 $workers); do
        (
            for i in $(seq 1 $files_per_worker); do
                curl -s "${base_url}/files?path=/conc-$w-$i.txt" >/dev/null 2>&1
            done
        ) &
    done

    wait

    local end=$(date +%s%3N)
    local read_time=$((end - start))

    echo "  读取: ${read_time}ms ($(($total_files * 1000 / read_time)) ops/sec)"

    # 清理
    for w in $(seq 1 $workers); do
        for i in $(seq 1 $files_per_worker); do
            curl -s -X DELETE "${base_url}/files?path=/conc-$w-$i.txt" >/dev/null 2>&1
        done
    done
}

# 测试目录结构深度
test_directory_depth() {
    local base_url=$1
    local name=$2
    local max_depth=$3

    log_info "[$name] 目录深度测试 (深度 $max_depth)..."

    local start=$(date +%s%3N)

    # 创建深层目录
    local current_path=""
    for depth in $(seq 1 $max_depth); do
        current_path="$current_path/level-$depth"
        curl -s -X POST "${base_url}/directories?path=${current_path}" >/dev/null 2>&1

        # 在每一层创建一个文件
        curl -s -X POST "${base_url}/files?path=${current_path}/file.txt" >/dev/null 2>&1
        echo "depth-$depth" | curl -s -X PUT "${base_url}/files?path=${current_path}/file.txt" --data-binary @- >/dev/null 2>&1
    done

    local end=$(date +%s%3N)
    local create_time=$((end - start))

    echo "  创建: ${create_time}ms"

    # 读取最深层文件
    local start=$(date +%s%3N)
    curl -s "${base_url}/files?path=${current_path}/file.txt" >/dev/null 2>&1
    local end=$(date +%s%3N)
    local deep_read_time=$((end - start))

    echo "  读取最深层文件: ${deep_read_time}ms"

    # 清理
    curl -s -X DELETE "${base_url}/directories?path=/level-1?recursive=true" >/dev/null 2>&1
    for depth in $(seq 1 $max_depth); do
        curl -s -X DELETE "${base_url}/directories?path=/level-$depth" >/dev/null 2>&1
    done
}

# 主测试流程
main() {
    echo "=========================================="
    echo "     AGFS 大规模性能测试套件"
    echo "=========================================="
    echo ""

    # 验证服务运行
    log_info "验证服务状态..."
    if ! curl -s http://localhost:8080/api/v1/health >/dev/null 2>&1; then
        log_error "Rust AGFS (8080) 未运行"
        exit 1
    fi
    if ! curl -s http://localhost:8081/api/v1/health >/dev/null 2>&1; then
        log_error "Go AGFS (8081) 未运行"
        exit 1
    fi
    echo ""

    # 测试 1: 不同文件大小
    echo "=========================================="
    echo "  测试 1: 文件大小影响"
    echo "=========================================="
    echo ""
    test_file_sizes "http://localhost:8080/api/v1" "RUST"
    echo ""
    test_file_sizes "http://localhost:8081/api/v1" "GO"
    echo ""

    # 测试 2: 大量小文件
    echo "=========================================="
    echo "  测试 2: 大量文件测试"
    echo "=========================================="
    echo ""

    for count in "${FILE_COUNTS[@]}"; do
        echo "--- $count 文件 ---"
        test_many_files "http://localhost:8080/api/v1" "RUST" $count
        test_many_files "http://localhost:8081/api/v1" "GO" $count
        echo ""
    done

    # 测试 3: 高并发
    echo "=========================================="
    echo "  测试 3: 高并发性能"
    echo "=========================================="
    echo ""

    test_concurrent "http://localhost:8080/api/v1" "RUST" 10 50
    test_concurrent "http://localhost:8081/api/v1" "GO" 10 50
    echo ""

    test_concurrent "http://localhost:8080/api/v1" "RUST" 50 20
    test_concurrent "http://localhost:8081/api/v1" "GO" 50 20
    echo ""

    # 测试 4: 目录深度
    echo "=========================================="
    echo "  测试 4: 目录结构深度"
    echo "=========================================="
    echo ""

    test_directory_depth "http://localhost:8080/api/v1" "RUST" 20
    test_directory_depth "http://localhost:8081/api/v1" "GO" 20
    echo ""

    # 最终总结
    echo "=========================================="
    echo "  测试完成"
    echo "=========================================="
}

main "$@"
