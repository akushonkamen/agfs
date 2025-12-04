// Example: Using HTTP client in C++ WASM plugin
//
// This example shows how to make HTTP requests from a C++ WASM plugin.

#include "agfs.h"
#include <iostream>

class HttpTestFS : public agfs::FileSystem {
public:
    const char* name() const override {
        return "httptestfs";
    }

    const char* readme() const override {
        return "HTTP Test Filesystem - Demonstrates HTTP requests from C++ WASM\n"
               "\n"
               "cat /test_get - Make a GET request to example.com\n"
               "cat /test_json - Fetch JSON from an API\n";
    }

    agfs::Result<agfs::FileInfo> stat(const std::string& path) override {
        if (path == "/") {
            return agfs::FileInfo::dir("", 0755);
        } else if (path == "/test_get" || path == "/test_json") {
            return agfs::FileInfo::file(path.substr(1), 0, 0644);
        }
        return agfs::Error::not_found();
    }

    agfs::Result<std::vector<agfs::FileInfo>> readdir(const std::string& path) override {
        if (path == "/") {
            return std::vector<agfs::FileInfo>{
                agfs::FileInfo::file("test_get", 0, 0644),
                agfs::FileInfo::file("test_json", 0, 0644)
            };
        }
        return agfs::Error::not_found();
    }

    agfs::Result<std::vector<uint8_t>> read(const std::string& path,
                                            int64_t offset, int64_t size) override {
        if (path == "/test_get") {
            // Simple GET request
            auto result = agfs::Http::get("https://example.com");
            if (!result.is_ok()) {
                return result.error();
            }

            auto response = result.unwrap();
            if (!response.is_success()) {
                std::string err = "HTTP error: " + std::to_string(response.status_code);
                return agfs::Error::other(err);
            }

            return response.body;
        }
        else if (path == "/test_json") {
            // GET request with custom headers
            auto result = agfs::Http::request(
                agfs::HttpRequest::get("https://api.github.com/users/github")
                    .add_header("User-Agent", "AGFS-WASM-Plugin")
                    .add_header("Accept", "application/json")
            );

            if (!result.is_ok()) {
                return result.error();
            }

            auto response = result.unwrap();
            if (!response.is_success()) {
                std::string err = "HTTP error: " + std::to_string(response.status_code);
                return agfs::Error::other(err);
            }

            return response.body;
        }

        return agfs::Error::not_found();
    }
};

AGFS_EXPORT_PLUGIN(HttpTestFS);
