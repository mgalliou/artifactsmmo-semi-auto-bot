# \BadgesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_all_badges_badges_get**](BadgesApi.md#get_all_badges_badges_get) | **GET** /badges | Get All Badges
[**get_badge_badges_code_get**](BadgesApi.md#get_badge_badges_code_get) | **GET** /badges/{code} | Get Badge



## get_all_badges_badges_get

> models::DataPageBadgeSchema get_all_badges_badges_get(page, size)
Get All Badges

List of all badges.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageBadgeSchema**](DataPage_BadgeSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_badge_badges_code_get

> models::BadgeResponseSchema get_badge_badges_code_get(code)
Get Badge

Retrieve the details of a badge.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the badge. | [required] |

### Return type

[**models::BadgeResponseSchema**](BadgeResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

