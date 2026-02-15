# \AccountsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_account_accounts_create_post**](AccountsApi.md#create_account_accounts_create_post) | **POST** /accounts/create | Create Account
[**forgot_password_accounts_forgot_password_post**](AccountsApi.md#forgot_password_accounts_forgot_password_post) | **POST** /accounts/forgot_password | Forgot Password
[**get_account_accounts_account_get**](AccountsApi.md#get_account_accounts_account_get) | **GET** /accounts/{account} | Get Account
[**get_account_achievements_accounts_account_achievements_get**](AccountsApi.md#get_account_achievements_accounts_account_achievements_get) | **GET** /accounts/{account}/achievements | Get Account Achievements
[**get_account_characters_accounts_account_characters_get**](AccountsApi.md#get_account_characters_accounts_account_characters_get) | **GET** /accounts/{account}/characters | Get Account Characters
[**reset_password_accounts_reset_password_post**](AccountsApi.md#reset_password_accounts_reset_password_post) | **POST** /accounts/reset_password | Reset Password



## create_account_accounts_create_post

> models::ResponseSchema create_account_accounts_create_post(add_account_schema)
Create Account

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**add_account_schema** | [**AddAccountSchema**](AddAccountSchema.md) |  | [required] |

### Return type

[**models::ResponseSchema**](ResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## forgot_password_accounts_forgot_password_post

> models::PasswordResetResponseSchema forgot_password_accounts_forgot_password_post(password_reset_request_schema)
Forgot Password

Request a password reset.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**password_reset_request_schema** | [**PasswordResetRequestSchema**](PasswordResetRequestSchema.md) |  | [required] |

### Return type

[**models::PasswordResetResponseSchema**](PasswordResetResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_account_accounts_account_get

> models::AccountDetailsSchema get_account_accounts_account_get(account)
Get Account

Retrieve the details of an account.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**account** | **String** | The name of the account. | [required] |

### Return type

[**models::AccountDetailsSchema**](AccountDetailsSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_account_achievements_accounts_account_achievements_get

> models::DataPageAccountAchievementSchema get_account_achievements_accounts_account_achievements_get(account, r#type, completed, page, size)
Get Account Achievements

Retrieve the achievements of a account.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**account** | **String** | The name of the account. | [required] |
**r#type** | Option<**String**> | Type of achievements. |  |
**completed** | Option<**bool**> | Filter by completed achievements. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageAccountAchievementSchema**](DataPage_AccountAchievementSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_account_characters_accounts_account_characters_get

> models::CharactersListSchema get_account_characters_accounts_account_characters_get(account)
Get Account Characters

Account character lists.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**account** | **String** | The name of the account. | [required] |

### Return type

[**models::CharactersListSchema**](CharactersListSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## reset_password_accounts_reset_password_post

> models::PasswordResetResponseSchema reset_password_accounts_reset_password_post(password_reset_confirm_schema)
Reset Password

Reset password with a token. Use /forgot_password to get a token by email.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**password_reset_confirm_schema** | [**PasswordResetConfirmSchema**](PasswordResetConfirmSchema.md) |  | [required] |

### Return type

[**models::PasswordResetResponseSchema**](PasswordResetResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

