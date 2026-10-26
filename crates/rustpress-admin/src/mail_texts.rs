//! The texts of the e-mail WordPress sends, as they are written in its source (they are the keys of the translation).

pub const PASSWORD_CHANGED: &str = r#"Hi ###USERNAME###,

This notice confirms that your password was changed on ###SITENAME###.

If you did not change your password, please contact the Site Administrator at
###ADMIN_EMAIL###

This email has been sent to ###EMAIL###

Regards,
All at ###SITENAME###
###SITEURL###"#;

pub const EMAIL_CHANGED: &str = r#"Hi ###USERNAME###,

This notice confirms that your email address on ###SITENAME### was changed to ###NEW_EMAIL###.

If you did not change your email, please contact the Site Administrator at
###ADMIN_EMAIL###

This email has been sent to ###EMAIL###

Regards,
All at ###SITENAME###
###SITEURL###"#;

pub const REQUEST_CONFIRMED_ADMIN: &str = r#"Howdy,

A user data privacy request has been confirmed on ###SITENAME###:

User: ###USER_EMAIL###
Request: ###DESCRIPTION###

You can view and manage these data privacy requests here:

###MANAGE_URL###

Regards,
All at ###SITENAME###
###SITEURL###"#;

pub const ERASURE_PLAIN: &str = r#"Howdy,

Your request to erase your personal data on ###SITENAME### has been completed.

If you have any follow-up questions or concerns, please contact the site administrator.

Regards,
All at ###SITENAME###
###SITEURL###"#;

pub const ERASURE_WITH_POLICY: &str = r#"Howdy,

Your request to erase your personal data on ###SITENAME### has been completed.

If you have any follow-up questions or concerns, please contact the site administrator.

For more information, you can also read our privacy policy: ###PRIVACY_POLICY_URL###

Regards,
All at ###SITENAME###
###SITEURL###"#;

pub const CONFIRM_ACTION: &str = r#"Howdy,

A request has been made to perform the following action on your account:

     ###DESCRIPTION###

To confirm this, please click on the following link:
###CONFIRM_URL###

You can safely ignore and delete this email if you do not want to
take this action.

Regards,
All at ###SITENAME###
###SITEURL###"#;

pub const EXPORT_EMAIL: &str = r#"Howdy,

Your request for an export of personal data has been completed. You may
download your personal data by clicking on the link below. For privacy
and security, we will automatically delete the file on ###EXPIRATION###,
so please download it before then.

###LINK###

Regards,
All at ###SITENAME###
###SITEURL###"#;

pub const NEW_ADMIN_EMAIL: &str = r#"Howdy,

A site administrator (###USERNAME###) recently requested to have the
administration email address changed on this site:
###SITEURL###

To confirm this change, please click on the following link:
###ADMIN_URL###

You can safely ignore and delete this email if you do not want to
take this action.

This email has been sent to ###EMAIL###

Regards,
All at ###SITENAME###
###SITEURL###"#;

pub const NEW_USER_EMAIL: &str = r#"Howdy ###USERNAME###,

You recently requested to have the email address on your account changed.

If this is correct, please click on the following link to change it:
###ADMIN_URL###

You can safely ignore and delete this email if you do not want to
take this action.

This email has been sent to ###EMAIL###

Regards,
All at ###SITENAME###
###SITEURL###"#;
