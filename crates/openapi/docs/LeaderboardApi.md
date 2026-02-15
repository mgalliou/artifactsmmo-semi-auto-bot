# \LeaderboardApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_accounts_leaderboard_leaderboard_accounts_get**](LeaderboardApi.md#get_accounts_leaderboard_leaderboard_accounts_get) | **GET** /leaderboard/accounts | Get Accounts Leaderboard
[**get_characters_leaderboard_leaderboard_characters_get**](LeaderboardApi.md#get_characters_leaderboard_leaderboard_characters_get) | **GET** /leaderboard/characters | Get Characters Leaderboard



## get_accounts_leaderboard_leaderboard_accounts_get

> models::DataPageAccountLeaderboardSchema get_accounts_leaderboard_leaderboard_accounts_get(sort, name, page, size)
Get Accounts Leaderboard

Fetch leaderboard details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**sort** | Option<[**models::AccountLeaderboardType**](.md)> | Sort of account leaderboards. |  |
**name** | Option<**String**> | Account name. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageAccountLeaderboardSchema**](DataPage_AccountLeaderboardSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_characters_leaderboard_leaderboard_characters_get

> models::DataPageCharacterLeaderboardSchema get_characters_leaderboard_leaderboard_characters_get(sort, name, page, size)
Get Characters Leaderboard

Fetch leaderboard details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**sort** | Option<[**models::CharacterLeaderboardType**](.md)> | Sort of character leaderboards. |  |
**name** | Option<**String**> | Character name. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageCharacterLeaderboardSchema**](DataPage_CharacterLeaderboardSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

