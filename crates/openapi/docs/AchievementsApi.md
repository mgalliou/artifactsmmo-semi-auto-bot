# \AchievementsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_achievement_achievements_code_get**](AchievementsApi.md#get_achievement_achievements_code_get) | **GET** /achievements/{code} | Get Achievement
[**get_all_achievements_achievements_get**](AchievementsApi.md#get_all_achievements_achievements_get) | **GET** /achievements | Get All Achievements



## get_achievement_achievements_code_get

> models::AchievementResponseSchema get_achievement_achievements_code_get(code)
Get Achievement

Retrieve the details of an achievement.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the achievement. | [required] |

### Return type

[**models::AchievementResponseSchema**](AchievementResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_all_achievements_achievements_get

> models::DataPageAchievementSchema get_all_achievements_achievements_get(r#type, page, size)
Get All Achievements

List of all achievements.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**r#type** | Option<[**models::AchievementType**](.md)> | Type of achievements. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageAchievementSchema**](DataPage_AchievementSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

