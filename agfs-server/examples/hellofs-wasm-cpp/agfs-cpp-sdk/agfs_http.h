#ifndef AGFS_HTTP_H
#define AGFS_HTTP_H

#include "agfs_types.h"
#include <string>
#include <vector>
#include <map>
#include <cstdint>

namespace agfs {

// Forward declarations for FFI functions
extern "C" {
    uint64_t host_http_request(const char* request_json);
}

// HTTP request builder
class HttpRequest {
public:
    std::string method = "GET";
    std::string url;
    std::map<std::string, std::string> headers;
    std::vector<uint8_t> body;
    int timeout = 30; // seconds

    HttpRequest() = default;

    static HttpRequest get(const std::string& url) {
        HttpRequest req;
        req.method = "GET";
        req.url = url;
        return req;
    }

    static HttpRequest post(const std::string& url) {
        HttpRequest req;
        req.method = "POST";
        req.url = url;
        return req;
    }

    static HttpRequest put(const std::string& url) {
        HttpRequest req;
        req.method = "PUT";
        req.url = url;
        return req;
    }

    static HttpRequest del(const std::string& url) {
        HttpRequest req;
        req.method = "DELETE";
        req.url = url;
        return req;
    }

    HttpRequest& set_method(const std::string& m) {
        method = m;
        return *this;
    }

    HttpRequest& add_header(const std::string& key, const std::string& value) {
        headers[key] = value;
        return *this;
    }

    HttpRequest& set_body(const std::vector<uint8_t>& data) {
        body = data;
        return *this;
    }

    HttpRequest& set_body(const std::string& data) {
        body.assign(data.begin(), data.end());
        return *this;
    }

    HttpRequest& set_timeout(int seconds) {
        timeout = seconds;
        return *this;
    }

    // Convert to JSON for FFI
    std::string to_json() const {
        std::string json = "{";
        json += "\"method\":\"" + method + "\",";
        json += "\"url\":\"" + url + "\",";
        json += "\"headers\":{";
        bool first = true;
        for (const auto& [key, value] : headers) {
            if (!first) json += ",";
            json += "\"" + key + "\":\"" + value + "\"";
            first = false;
        }
        json += "},";
        json += "\"body\":[";
        for (size_t i = 0; i < body.size(); i++) {
            if (i > 0) json += ",";
            json += std::to_string(body[i]);
        }
        json += "],";
        json += "\"timeout\":" + std::to_string(timeout);
        json += "}";
        return json;
    }
};

// HTTP response
class HttpResponse {
public:
    int status_code = 0;
    std::map<std::string, std::string> headers;
    std::vector<uint8_t> body;
    std::string error;

    bool is_success() const {
        return status_code >= 200 && status_code < 300;
    }

    bool has_error() const {
        return !error.empty();
    }

    std::string text() const {
        return std::string(body.begin(), body.end());
    }

    // Simple base64 decode
    static std::vector<uint8_t> base64_decode(const std::string& input) {
        static const uint8_t base64_table[128] = {
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 62, 255, 255, 255, 63,
            52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 255, 255, 255, 0, 255, 255,
            255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
            15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 255, 255, 255, 255, 255,
            255, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
            41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 255, 255, 255, 255, 255,
        };

        std::vector<uint8_t> output;
        if (input.empty()) return output;

        output.reserve((input.size() * 3) / 4);
        uint32_t buf = 0;
        int bits = 0;

        for (char c : input) {
            if (c == '=') break;
            if (c >= 128 || base64_table[c] == 255) continue;

            buf = (buf << 6) | base64_table[c];
            bits += 6;

            if (bits >= 8) {
                bits -= 8;
                output.push_back((buf >> bits) & 0xFF);
                buf &= (1 << bits) - 1;
            }
        }

        return output;
    }

    // Parse from JSON response
    static Result<HttpResponse> from_json(const std::string& json) {
        HttpResponse resp;

        // Simple JSON parsing (for production, use a real JSON library)
        size_t pos = 0;

        // Parse status_code
        pos = json.find("\"status_code\":");
        if (pos != std::string::npos) {
            pos += 14;
            while (pos < json.size() && (json[pos] == ' ' || json[pos] == ':')) pos++;
            resp.status_code = std::atoi(json.c_str() + pos);
        }

        // Parse body (base64 string)
        pos = json.find("\"body\":\"");
        if (pos != std::string::npos) {
            pos += 8;
            size_t end = json.find("\"", pos);
            if (end != std::string::npos) {
                std::string body_b64 = json.substr(pos, end - pos);
                resp.body = base64_decode(body_b64);
            }
        }

        // Parse error
        pos = json.find("\"error\":\"");
        if (pos != std::string::npos) {
            pos += 9;
            size_t end = json.find("\"", pos);
            if (end != std::string::npos) {
                resp.error = json.substr(pos, end - pos);
            }
        }

        if (!resp.error.empty()) {
            return Error::other(resp.error);
        }

        return resp;
    }
};

// HTTP client
class Http {
public:
    static Result<HttpResponse> request(const HttpRequest& req) {
        std::string request_json = req.to_json();

        uint64_t result = host_http_request(request_json.c_str());

        // Unpack: lower 32 bits = pointer, upper 32 bits = size
        uint32_t response_ptr = result & 0xFFFFFFFF;
        uint32_t response_size = (result >> 32) & 0xFFFFFFFF;

        if (response_ptr == 0) {
            return Error::other("HTTP request failed");
        }

        // Read response from memory
        const char* response_data = reinterpret_cast<const char*>(response_ptr);
        std::string response_json(response_data, response_size);

        return HttpResponse::from_json(response_json);
    }

    static Result<HttpResponse> get(const std::string& url) {
        return request(HttpRequest::get(url));
    }

    static Result<HttpResponse> post(const std::string& url, const std::vector<uint8_t>& body) {
        return request(HttpRequest::post(url).set_body(body));
    }

    static Result<HttpResponse> post(const std::string& url, const std::string& body) {
        return request(HttpRequest::post(url).set_body(body));
    }

    static Result<HttpResponse> put(const std::string& url, const std::vector<uint8_t>& body) {
        return request(HttpRequest::put(url).set_body(body));
    }

    static Result<HttpResponse> del(const std::string& url) {
        return request(HttpRequest::del(url));
    }
};

} // namespace agfs

#endif // AGFS_HTTP_H
